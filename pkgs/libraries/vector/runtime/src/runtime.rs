use std::collections::{HashMap, HashSet};
use std::io::{Error, ErrorKind};
use std::panic::AssertUnwindSafe;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;

use tokio::sync::Semaphore;
use tokio::sync::{mpsc, watch};
use tokio::task::{JoinHandle, spawn};
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

use futures::FutureExt;
use log::info;

use crate::{Component, ComponentType, Event, Message};

struct Bootstrap {
    component: RwLock<Arc<dyn Component>>,
    boot: Arc<Semaphore>,
    cancel: CancellationToken,
    signal_tx: watch::Sender<()>,
    signal_rx: watch::Receiver<()>,
    report: mpsc::Sender<Event>,
    receiver: RwLock<Option<mpsc::Receiver<Message>>>,
    fanout: Arc<RwLock<HashMap<usize, mpsc::Sender<Message>>>>,
    mapping: Arc<RwLock<HashMap<usize, Vec<usize>>>>,
}

impl Bootstrap {
    pub fn new(
        boot: Arc<Semaphore>,
        component: &Arc<dyn Component>,
        rx: mpsc::Receiver<Message>,
        fanout: Arc<RwLock<HashMap<usize, mpsc::Sender<Message>>>>,
        mapping: Arc<RwLock<HashMap<usize, Vec<usize>>>>,
        report: mpsc::Sender<Event>,
    ) -> Self {
        let (signal_tx, signal_rx) = watch::channel(());

        Self {
            component: RwLock::new(component.clone()),
            cancel: CancellationToken::new(),
            receiver: RwLock::new(Some(rx)),
            report,
            fanout,
            mapping,
            signal_rx,
            signal_tx,
            boot,
        }
    }

    pub fn id(&self) -> Result<String, Error> {
        self.component
            .read()
            .map_err(|error| {
                Error::new(
                    ErrorKind::BrokenPipe,
                    format!("Component not readable: {}", error),
                )
            })
            .map(|component| component.id())
    }

    pub fn reload(&self, component: &Arc<dyn Component>) -> Result<(), Error> {
        let mut lock = self
            .component
            .write()
            .map_err(|_| Error::new(ErrorKind::Other, "Lock poison"))?;
        *lock = component.clone();

        self.signal_tx.send(()).map_err(|error| {
            Error::new(
                ErrorKind::ConnectionRefused,
                format!("Component not readable: {}", error),
            )
        })?;
        Ok(())
    }

    pub fn stop(&self) -> Result<(), Error> {
        if self.cancel.is_cancelled() {
            return Err(Error::new(
                ErrorKind::ConnectionRefused,
                "Component has been closed",
            ));
        }

        self.cancel.cancel();
        Ok(())
    }

    pub fn compare(&self, component: &Arc<dyn Component>) -> Result<bool, Error> {
        Ok(self
            .component
            .read()
            .map_err(|error| {
                Error::new(
                    ErrorKind::BrokenPipe,
                    format!("component read error: {:?}", error),
                )
            })?
            .compare(component.as_ref()))
    }

    fn get_senders(&self, id: usize) -> Result<Vec<mpsc::Sender<Message>>, Error> {
        let mut txs = Vec::new();

        let outputs = self
            .mapping
            .read()
            .map_err(|e| Error::new(ErrorKind::BrokenPipe, format!("Mapping read error: {}", e)))?
            .get(&id)
            .cloned()
            .unwrap_or_else(|| Vec::new());

        let fanout = self
            .fanout
            .read()
            .map_err(|e| Error::new(ErrorKind::BrokenPipe, format!("Fanout read error: {}", e)))?;

        for output_id in outputs {
            if let Some(sender) = fanout.get(&output_id) {
                if sender.is_closed() {
                    return Err(Error::new(
                        ErrorKind::BrokenPipe,
                        format!("Sender {} has been closed unexpectedly", output_id),
                    ));
                }

                txs.push(sender.clone());
            } else {
                return Err(Error::new(
                    ErrorKind::BrokenPipe,
                    format!("Sender not found for downstream node {}", output_id),
                ));
            }
        }

        Ok(txs)
    }

    async fn execute(&self, id: usize) -> Result<(), Error> {
        let _permit = self
            .boot
            .acquire()
            .await
            .map_err(|_| Error::new(ErrorKind::Interrupted, "Boot interrupted"))?;

        info!("Component with id {} is on running", id);

        let mut signal = self.signal_rx.clone();
        let cancel = self.cancel.clone();

        let mut rx = {
            let mut rx_guard = self
                .receiver
                .write()
                .map_err(|_| Error::new(ErrorKind::Other, "Lock poisoned"))?;

            rx_guard
                .take()
                .ok_or_else(|| Error::new(ErrorKind::Other, "No receiver available"))?
        };

        if rx.is_closed() {
            return Err(Error::new(
                ErrorKind::BrokenPipe,
                format!("Receiver {} has been closed unexpectedly", id),
            ));
        }

        loop {
            let component = self
                .component
                .read()
                .map_err(|error| {
                    Error::new(
                        ErrorKind::BrokenPipe,
                        format!("Failed to read `component`: {}", error),
                    )
                })?
                .clone();
            let report = self.report.clone();
            let txs = self.get_senders(id)?;
            let run_in_future =
                AssertUnwindSafe(component.run(id, &mut rx, &txs, &report)).catch_unwind();

            tokio::select! {
                panic_res = run_in_future => {
                    match panic_res {
                        Ok(Ok(())) => return Ok(()),
                        Ok(Err(issue)) => {
                            report.send(Event::Major((id, issue)))
                                .await
                                .map_err(|error| Error::new(
                                    ErrorKind::BrokenPipe,
                                    format!(
                                        "Failed to send issue: {}",
                                        error,
                                    ),
                                ))?;
                            sleep(Duration::from_secs(1)).await;
                            continue;
                        }
                        Err(panic) => {
                            report.send(Event::Major((
                                    id,
                                    Error::new(
                                        ErrorKind::BrokenPipe,
                                        format!("Panic at node {}:\n {:?}", id, panic),
                                    ),
                                )))
                                .await
                                .map_err(|error| Error::new(
                                    ErrorKind::BrokenPipe,
                                    format!(
                                        "Failed to send issue: {}",
                                        error,
                                    ),
                                ))?;
                            sleep(Duration::from_secs(1)).await;
                            continue;
                        }
                    }
                }

                _ = cancel.cancelled() => {
                    return Ok(());
                }

                _ = signal.changed() => {
                    continue;
                }
            }
        }
    }
}

pub struct Runtime {
    // @NOTE: idea
    //
    // outputs: vec[vec[int]]
    // nodes: HashMap[String, int]
    //
    // a - a -  a
    //   \     /
    //    a - a
    //
    // Base on graph schema above, we can think about incremental
    // validation where when we change anything, the Runtime will easily
    // detect whether or not the graph is broken and cannot stream any
    // more.

    // @NOTE: runtime management
    senders: Arc<RwLock<HashMap<usize, mpsc::Sender<Message>>>>,
    boot: Arc<Semaphore>,
    tasks: RwLock<HashMap<usize, JoinHandle<Result<(), Error>>>>,
    bootstraps: RwLock<HashMap<usize, Arc<Bootstrap>>>,
    report_tx: mpsc::Sender<Event>,
    report_rx: Option<mpsc::Receiver<Event>>,
    is_started: bool,

    // @NOTE: topology management
    outputs: Arc<RwLock<HashMap<usize, Vec<usize>>>>,
    inputs: RwLock<HashMap<usize, Vec<usize>>>,
    nodes: RwLock<HashMap<String, usize>>,
    inc: AtomicUsize,
}

impl Runtime {
    pub fn new() -> Self {
        let (report_tx, report_rx) = mpsc::channel(100);
        let report_rx = Some(report_rx);
        Self {
            // @NOTE: declare topology
            outputs: Arc::new(RwLock::new(HashMap::new())),
            inputs: RwLock::new(HashMap::new()),
            nodes: RwLock::new(HashMap::new()),
            inc: AtomicUsize::new(0),
            is_started: false,
            report_tx,
            report_rx,

            // @NOTE: declare runtime self-management
            boot: Arc::new(Semaphore::new(0)),
            tasks: RwLock::new(HashMap::new()),
            senders: Arc::new(RwLock::new(HashMap::new())),
            bootstraps: RwLock::new(HashMap::new()),
        }
    }

    pub fn index(&self, id: String) -> Result<usize, Error> {
        let nodes = self.nodes.read().map_err(|error| {
            Error::new(
                ErrorKind::BrokenPipe,
                format!("Failed to read from nodes: {}", error),
            )
        })?;

        Ok(*nodes
            .get(&id)
            .ok_or_else(|| Error::new(ErrorKind::InvalidInput, format!("Not found `{id}`")))?)
    }

    pub async fn inject(&self, id: String, msg: Message) -> Result<(), Error> {
        let sender = {
            let nodes = self
                .nodes
                .read()
                .map_err(|e| Error::new(ErrorKind::BrokenPipe, format!("Nodes lock error: {e}")))?;
            let inputs = self.inputs.read().map_err(|e| {
                Error::new(ErrorKind::BrokenPipe, format!("Inputs lock error: {e}"))
            })?;
            let senders = self.senders.read().map_err(|e| {
                Error::new(ErrorKind::BrokenPipe, format!("Senders lock error: {e}"))
            })?;

            let idx = nodes.get(&id).ok_or_else(|| {
                Error::new(
                    ErrorKind::InvalidData,
                    format!("Not found node with id {id}"),
                )
            })?;

            if inputs.contains_key(idx) {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    format!("Node {id} must be a source node"),
                ));
            }

            senders
                .get(idx)
                .cloned() // Clone cái Sender (Tokio mpsc sender clone rất rẻ)
                .ok_or_else(|| {
                    Error::new(
                        ErrorKind::BrokenPipe,
                        format!("Node {id} doesn't have any senders"),
                    )
                })
        }?;

        sender.send(msg).await.map_err(|error| {
            Error::new(
                ErrorKind::BrokenPipe,
                format!("Send data to node {id} failed: {error}"),
            )
        })?;

        Ok(())
    }

    pub fn reload(&self, components: Vec<Arc<dyn Component>>) -> Result<(), Error> {
        let (adds, diffs, dels) = {
            let nodes = self.nodes.read().map_err(|error| {
                Error::new(
                    ErrorKind::BrokenPipe,
                    format!("Failed reading nodes: {}", error),
                )
            })?;
            let bootstraps = self.bootstraps.read().map_err(|error| {
                Error::new(
                    ErrorKind::BrokenPipe,
                    format!("Failed reading bootstrap: {:?}", error),
                )
            })?;

            let mut diffs = Vec::new();
            let mut adds = Vec::new();
            let mut dels = bootstraps.keys().collect::<HashSet<_>>();

            for component in &components {
                if let Some(idx) = nodes.get(&component.id()) {
                    let bootstrap = bootstraps.get(&idx).ok_or_else(|| {
                        Error::new(
                            ErrorKind::BrokenPipe,
                            format!("Failed to get bootstrap with id {}", idx),
                        )
                    })?;

                    bootstrap.compare(&component).map(|is_same| {
                        if !is_same {
                            diffs.push(component);
                        }
                    })?;

                    dels.remove(&idx);
                } else {
                    adds.push(component);
                }
            }

            (
                adds,
                diffs,
                dels.into_iter().map(|id| id.clone()).collect::<Vec<_>>(),
            )
        };

        self.validate_if_adding_new_nodes(&adds, &diffs)?;
        self.validate_if_changing_nodes(&diffs, &adds)?;
        self.validate_if_remove_outdated_nodes(&dels, &adds, &diffs)?;

        self.add_new_nodes(&adds)?;
        self.configure_links_after_adding_new_nodes(&adds)?;

        self.update_changing_nodes(&diffs)?;
        self.configure_links_after_changing_nodes(&diffs)?;

        self.remove_oudated_nodes(&dels)?;
        self.configure_links_after_remove_oudated_nodes(&dels)?;

        if self.is_started {
            let permit_count = self
                .bootstraps
                .read()
                .map_err(|e| Error::new(ErrorKind::BrokenPipe, e.to_string()))?
                .len();

            self.boot.add_permits(permit_count);
        }
        Ok(())
    }

    pub fn start<F, Fut>(&mut self, handler: F) -> Result<JoinHandle<()>, Error>
    where
        F: Fn(Event) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        if self.is_started {
            return Err(Error::new(ErrorKind::BrokenPipe, "already started"));
        }

        let mut rx = self
            .report_rx
            .take()
            .ok_or_else(|| Error::new(ErrorKind::Other, "receiver already taken"))?;

        let task_handler = tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                handler(event).await;
            }
        });

        let permit_count = self
            .bootstraps
            .read()
            .map_err(|e| Error::new(ErrorKind::BrokenPipe, e.to_string()))?
            .len();

        self.boot.add_permits(permit_count);
        self.is_started = true;
        Ok(task_handler)
    }

    pub fn stop(&self) -> Result<(), Error> {
        self.reload(Vec::new())
    }

    pub async fn wait_for_shutdown(&self) -> Result<(), Error> {
        let tasks: Vec<_> = {
            let mut tasks_guard = self.tasks.write().unwrap();
            tasks_guard.drain().map(|(_, handle)| handle).collect()
        };

        for handle in tasks {
            let _ = handle.await;
        }

        info!("All component tasks have been shut down");
        Ok(())
    }

    fn add_new_nodes(&self, adds: &Vec<&Arc<dyn Component>>) -> Result<(), Error> {
        for component in adds {
            let idx = self.inc.fetch_add(1, Ordering::SeqCst);
            let (tx_data, rx_data) = mpsc::channel::<Message>(1024);

            self.nodes
                .write()
                .map_err(|error| {
                    Error::new(
                        ErrorKind::BrokenPipe,
                        format!("Failed to write to nodes: {}", error),
                    )
                })?
                .entry(component.id())
                .or_insert(idx);

            self.senders
                .write()
                .map_err(|error| {
                    Error::new(
                        ErrorKind::BrokenPipe,
                        format!("Failed to write to senders: {}", error),
                    )
                })?
                .entry(idx)
                .or_insert(tx_data.clone());

            let bootstrap = Arc::new(Bootstrap::new(
                self.boot.clone(),
                component,
                rx_data,
                self.senders.clone(),
                self.outputs.clone(),
                self.report_tx.clone(),
            ));

            self.bootstraps
                .write()
                .map_err(|error| {
                    Error::new(
                        ErrorKind::BrokenPipe,
                        format!("Fail writing bootstrap {:?}", error),
                    )
                })?
                .entry(idx)
                .or_insert(bootstrap.clone());

            info!("Component {} with id {} is starting", component.id(), idx);

            self.tasks
                .write()
                .map_err(|error| {
                    Error::new(
                        ErrorKind::BrokenPipe,
                        format!("Failed to write to tasks: {}", error),
                    )
                })?
                .insert(
                    idx,
                    spawn(async move {
                        let ret = bootstrap.execute(idx).await;
                        if let Err(error) = ret {
                            info!("Component with id {} is closed: {}", idx, error);
                            Err(error)
                        } else {
                            Ok(())
                        }
                    }),
                );
        }

        Ok(())
    }

    fn validate_if_adding_new_nodes(
        &self,
        adds: &Vec<&Arc<dyn Component>>,
        diffs: &Vec<&Arc<dyn Component>>,
    ) -> Result<(), Error> {
        let nodes = self.nodes.read().map_err(|error| {
            Error::new(
                ErrorKind::BrokenPipe,
                format!("Failed to write to nodes: {}", error),
            )
        })?;
        let will_add = adds
            .iter()
            .enumerate()
            .map(|(idx, component)| (component.id(), idx))
            .collect::<HashMap<_, _>>();
        let will_be_linked = diffs
            .iter()
            .filter_map(|component| component.get_inputs())
            .flatten()
            .cloned()
            .collect::<HashSet<_>>();

        let mut dead_nodes = will_add.clone();

        for component in adds {
            if component.component_type() != ComponentType::Source {
                if let Some(inputs) = component.get_inputs() {
                    if inputs.len() == 0 {
                        return Err(Error::new(
                            ErrorKind::InvalidData,
                            format!(
                                "Node '{}' requires to have at least one input",
                                component.id(),
                            ),
                        ));
                    }
                } else {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        format!(
                            "Node '{}' requires to not return None when listing inputs",
                            component.id(),
                        ),
                    ));
                }
            }

            if let Some(inputs) = component.get_inputs() {
                for input in inputs {
                    let exists_in_current = nodes.contains_key(input);
                    let exists_in_adds = will_add.contains_key(input);

                    if !exists_in_current && !exists_in_adds {
                        return Err(Error::new(
                            ErrorKind::InvalidData,
                            format!(
                                "Validation Failed: Node '{}' requires input '{}' but it's not found.",
                                component.id(),
                                input,
                            ),
                        ));
                    }

                    if let Some(&idx) = will_add.get(input) {
                        if adds[idx].component_type() == ComponentType::Sink {
                            return Err(Error::new(
                                ErrorKind::InvalidData,
                                format!("Node {} must not be Sink", adds[idx].id()),
                            ));
                        }

                        dead_nodes.remove(input);
                    }
                }
            } else if component.component_type() != ComponentType::Source {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    format!(
                        "Node {} must be Source, not {}",
                        component.id(),
                        component.component_type(),
                    ),
                ));
            }
        }

        for (_, idx) in dead_nodes {
            if adds[idx].component_type() != ComponentType::Sink {
                if !will_be_linked.contains(&adds[idx].id()) {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        format!(
                            "{} {} must have connect with another nodes",
                            adds[idx].component_type(),
                            adds[idx].id()
                        ),
                    ));
                }
            }
        }

        Ok(())
    }

    fn configure_links_after_adding_new_nodes(
        &self,
        adds: &Vec<&Arc<dyn Component>>,
    ) -> Result<(), Error> {
        let mut inputs_map = self.inputs.write().unwrap();
        let mut outputs_map = self.outputs.write().unwrap();
        let nodes = self.nodes.read().unwrap();

        for component in adds {
            let current_idx = *nodes.get(&component.id()).unwrap();

            if let Some(inputs) = component.get_inputs() {
                let mut input_indices = Vec::new();

                for input_name in inputs {
                    if let Some(&source_idx) = nodes.get(input_name) {
                        let outs = outputs_map.entry(source_idx).or_insert_with(Vec::new);
                        if !outs.contains(&current_idx) {
                            outs.push(current_idx);
                        }

                        if !input_indices.contains(&source_idx) {
                            input_indices.push(source_idx);
                        }
                    }
                }
                inputs_map.insert(current_idx, input_indices);
            }
        }
        Ok(())
    }

    fn validate_if_changing_nodes(
        &self,
        diffs: &Vec<&Arc<dyn Component>>,
        adds: &Vec<&Arc<dyn Component>>,
    ) -> Result<(), Error> {
        let nodes = self.nodes.read().unwrap();
        let add_ids: HashSet<String> = adds.iter().map(|c| c.id()).collect();

        for component in diffs {
            if let Some(inputs) = component.get_inputs() {
                for input_id in inputs {
                    if !nodes.contains_key(input_id) && !add_ids.contains(input_id) {
                        return Err(Error::new(
                            ErrorKind::InvalidData,
                            format!(
                                "Node {} wants to receive from non-existent node {}",
                                component.id(),
                                input_id,
                            ),
                        ));
                    }

                    if input_id == &component.id() {
                        return Err(Error::new(
                            ErrorKind::InvalidData,
                            "Self-loop is not allowed",
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    fn update_changing_nodes(&self, diffs: &Vec<&Arc<dyn Component>>) -> Result<(), Error> {
        for component in diffs {
            let idx = self
                .nodes
                .read()
                .map_err(|error| {
                    Error::new(
                        ErrorKind::BrokenPipe,
                        format!("Fail writing bootstrap {:?}", error),
                    )
                })?
                .get(&component.id())
                .ok_or_else(|| {
                    Error::new(
                        ErrorKind::BrokenPipe,
                        format!("Failed to get id of node {}", component.id()),
                    )
                })?
                .clone();

            self.bootstraps
                .write()
                .map_err(|error| {
                    Error::new(
                        ErrorKind::BrokenPipe,
                        format!("Fail writing bootstrap {:?}", error),
                    )
                })?
                .get(&idx)
                .ok_or_else(|| {
                    Error::new(
                        ErrorKind::BrokenPipe,
                        format!("Not found component with id {}", component.id()),
                    )
                })?
                .reload(component)?;
        }

        Ok(())
    }

    fn configure_links_after_changing_nodes(
        &self,
        diffs: &Vec<&Arc<dyn Component>>,
    ) -> Result<(), Error> {
        let mut inputs_map = self.inputs.write().unwrap();
        let mut outputs_map = self.outputs.write().unwrap();
        let nodes = self.nodes.read().unwrap();

        for component in diffs {
            let current_id = component.id();
            let current_idx = *nodes.get(&current_id).expect("Bug: Diff node not found");

            let new_input_indices: Vec<usize> = component
                .get_inputs()
                .map(|v| v.iter().filter_map(|id| nodes.get(id).cloned()).collect())
                .unwrap_or_default();

            let old_input_indices = inputs_map.get(&current_idx).cloned().unwrap_or_default();

            for old_source_idx in &old_input_indices {
                if !new_input_indices.contains(old_source_idx) {
                    if let Some(outs) = outputs_map.get_mut(old_source_idx) {
                        outs.retain(|&idx| idx != current_idx);
                    }
                }
            }

            for &new_source_idx in &new_input_indices {
                if !old_input_indices.contains(&new_source_idx) {
                    let outs = outputs_map.entry(new_source_idx).or_insert_with(Vec::new);
                    if !outs.contains(&current_idx) {
                        outs.push(current_idx);
                    }
                }
            }

            inputs_map.insert(current_idx, new_input_indices);
        }
        Ok(())
    }

    fn validate_if_remove_outdated_nodes(
        &self,
        dels: &Vec<usize>,
        adds: &Vec<&Arc<dyn Component>>,
        diffs: &Vec<&Arc<dyn Component>>,
    ) -> Result<(), Error> {
        let nodes = self.nodes.read().map_err(|error| {
            Error::new(
                ErrorKind::BrokenPipe,
                format!("Failed to write to nodes: {}", error),
            )
        })?;
        let inputs = self.inputs.read().map_err(|error| {
            Error::new(
                ErrorKind::BrokenPipe,
                format!("Fail reading inputs {:?}", error),
            )
        })?;
        let outputs = self.outputs.read().map_err(|error| {
            Error::new(
                ErrorKind::BrokenPipe,
                format!("Fail reading outputs {:?}", error),
            )
        })?;
        let will_change_input = diffs
            .iter()
            .map(|component| {
                let node_idx = nodes
                    .get(&component.id())
                    .cloned()
                    .expect("Never reach to this point or this is bug");

                let input_indices = component
                    .get_inputs()
                    .map(|v| {
                        v.iter()
                            .filter_map(|id| nodes.get(id).cloned())
                            .collect::<HashSet<_>>()
                    })
                    .unwrap_or_default();

                (node_idx, input_indices)
            })
            .collect::<HashMap<_, _>>();
        let will_delete = dels.iter().collect::<HashSet<_>>();
        let will_be_linked = adds
            .iter()
            .filter_map(|component| component.get_inputs())
            .flatten()
            .filter_map(|id| nodes.get(id))
            .cloned()
            .collect::<HashSet<_>>();

        for id_of_dead_node in dels {
            if will_be_linked.contains(id_of_dead_node) {
                // @TODO: mapping id to node name

                return Err(Error::new(
                    ErrorKind::InvalidData,
                    format!(
                        "You are adding new node to receive data from dead node {}",
                        id_of_dead_node,
                    ),
                ));
            }

            let mut sent_by_nodes = if let Some(input) = inputs.get(id_of_dead_node) {
                input.iter().collect::<HashSet<_>>()
            } else {
                HashSet::new()
            };

            if let Some(output) = outputs.get(id_of_dead_node) {
                for receiving_node_id in output {
                    if will_delete.contains(receiving_node_id) {
                        continue;
                    }

                    if let Some(changing_inputs) = will_change_input.get(receiving_node_id) {
                        if changing_inputs.contains(id_of_dead_node) {
                            // @TODO: mapping id to node name

                            return Err(Error::new(
                                ErrorKind::InvalidData,
                                format!(
                                    "Node {} mustn't receive transaction from {}",
                                    receiving_node_id, id_of_dead_node,
                                ),
                            ));
                        }

                        for input in changing_inputs {
                            sent_by_nodes.remove(input);
                        }
                    } else {
                        // @TODO: mapping id to node name

                        println!("{}", receiving_node_id);
                        return Err(Error::new(
                            ErrorKind::InvalidData,
                            format!(
                                "Node {} mustn't receive transaction from {}",
                                receiving_node_id, id_of_dead_node,
                            ),
                        ));
                    }
                }
            }

            if sent_by_nodes.len() > 0 {
                for node_id in sent_by_nodes {
                    if let Some(output) = outputs.get(node_id) {
                        if output.len() == 1 && !will_delete.contains(node_id) {
                            // @TODO: mapping id to node name

                            return Err(Error::new(
                                ErrorKind::InvalidData,
                                format!("Node {} is on deadline", node_id),
                            ));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn remove_oudated_nodes(&self, dels: &Vec<usize>) -> Result<(), Error> {
        for id in dels {
            let mut bootstraps = self.bootstraps.write().map_err(|error| {
                Error::new(
                    ErrorKind::BrokenPipe,
                    format!("Fail writing bootstrap {:?}", error),
                )
            })?;
            let mut tasks = self.tasks.write().map_err(|error| {
                Error::new(
                    ErrorKind::BrokenPipe,
                    format!("Fail writing bootstrap {:?}", error),
                )
            })?;
            let mut nodes = self.nodes.write().map_err(|error| {
                Error::new(
                    ErrorKind::BrokenPipe,
                    format!("Fail writing nodes {:?}", error),
                )
            })?;

            let name = bootstraps
                .get(&id)
                .ok_or_else(|| {
                    Error::new(
                        ErrorKind::BrokenPipe,
                        format!("Not found component with id {}", id),
                    )
                })?
                .id()
                .map_err(|error| {
                    Error::new(
                        ErrorKind::BrokenPipe,
                        format!("Fail query node name with id {}: {:?}", id, error),
                    )
                })?;

            bootstraps
                .get(&id)
                .ok_or_else(|| {
                    Error::new(
                        ErrorKind::BrokenPipe,
                        format!("Not found component with id {}", id),
                    )
                })?
                .stop()?;
            bootstraps.remove(&id);
            tasks.remove(&id);
            nodes.remove(&name);
        }

        Ok(())
    }

    fn configure_links_after_remove_oudated_nodes(&self, dels: &Vec<usize>) -> Result<(), Error> {
        let mut inputs_map = self
            .inputs
            .write()
            .map_err(|e| Error::new(ErrorKind::BrokenPipe, e.to_string()))?;
        let mut outputs_map = self
            .outputs
            .write()
            .map_err(|e| Error::new(ErrorKind::BrokenPipe, e.to_string()))?;

        for &id_of_dead_node in dels {
            if let Some(upstream_indices) = inputs_map.get(&id_of_dead_node) {
                for &source_idx in upstream_indices {
                    if let Some(outs) = outputs_map.get_mut(&source_idx) {
                        outs.retain(|&idx| idx != id_of_dead_node);
                    }
                }
            }

            inputs_map.remove(&id_of_dead_node);
            outputs_map.remove(&id_of_dead_node);
        }

        Ok(())
    }
}
