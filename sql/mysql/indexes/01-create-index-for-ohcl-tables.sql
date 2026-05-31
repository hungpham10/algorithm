CREATE INDEX idx_mapping_symbol ON ohcl_mapping_product_in_store_to_symbol(symbol);
CREATE INDEX idx_mapping_store ON ohcl_mapping_product_in_store_to_symbol(store);
CREATE INDEX idx_anchor_symbol_store ON ohcl_product_anchors(symbol, store);
CREATE INDEX idx_symbols_broker_id_id ON ohcl_symbols (broker_id, id);
CREATE INDEX idx_product_anchors_symbol_scope ON ohcl_product_anchors (symbol, scope);
CREATE INDEX idx_mapping_product_anchor_id ON mapping_product_in_store_to_symbol (product_anchor_id);
