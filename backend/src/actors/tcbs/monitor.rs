use prometheus::IntCounterVec;
use std::sync::Arc;

use super::Order;

#[cfg(not(feature = "python"))]
#[inline]
pub fn monitor_order_flow(symbol: &str, order: &Order, order_counter: Arc<IntCounterVec>) {
    order_counter.with_label_values(&[symbol]).inc_by(order.v);
}
