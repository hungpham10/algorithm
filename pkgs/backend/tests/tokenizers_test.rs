mod tests {
    use anyhow::Result;
    use candle_core::{Device, Tensor};
    use candle_nn::VarBuilder;
    use candle_transformers::models::bert::{BertModel, Config, DTYPE};
    use hf_hub::{api::sync::Api, Repo, RepoType};
    use tokenizers::Tokenizer;

    #[test]
    fn test_encoding() {
        //dotenvy::dotenv().ok();
        //env_logger::init();

        //let mut config: Config = serde_json::from_str(
        //    std::fs::read_to_string("/tmp/config.json")
        //        .unwrap()
        //        .as_str(),
        //)
        //.unwrap();
        //let tokens = Tokenizer::from_file("/tmp/tokenizer.json")
        //    .map_err(anyhow::Error::msg)
        //    .unwrap()
        //    .with_padding(None)
        //    .with_truncation(None)
        //    .map_err(anyhow::Error::msg)
        //    .unwrap()
        //    .encode("xin chào", true)
        //    .map_err(anyhow::Error::msg)
        //    .unwrap()
        //    .get_ids()
        //    .to_vec();
        //let vb = VarBuilder::from_pth("/tmp/pytorch_model.bin", DTYPE, &Device::Cpu).unwrap();
        //let model = BertModel::load(vb, &config).unwrap();
        //let token_ids = Tensor::new(&tokens[..], &Device::Cpu)
        //    .unwrap()
        //    .unsqueeze(0)
        //    .unwrap();
        //let token_type_ids = token_ids.zeros_like().unwrap();

        //println!(
        //    "{:?} {}",
        //    tokens,
        //    model.forward(&token_ids, &token_type_ids, None).unwrap(),
        //);
    }
}
