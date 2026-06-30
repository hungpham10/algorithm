<script>
    import DefaultConfig from "./DefaultConfig.svelte";
    let { params = {}, paramDefs = [], onUpdate = () => {} } = $props();
</script>
<DefaultConfig {params} {paramDefs} {onUpdate} />
