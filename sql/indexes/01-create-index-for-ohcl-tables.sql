CREATE INDEX idx_mapping_symbol ON ohcl_mapping_product_in_store_to_symbol(symbol);
CREATE INDEX idx_mapping_store ON ohcl_mapping_product_in_store_to_symbol(store);
CREATE INDEX idx_anchor_symbol_store ON ohcl_product_anchors(symbol, store);
