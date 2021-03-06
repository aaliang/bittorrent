use std::collections::HashMap;
use url::percent_encoding::{utf8_percent_encode, percent_encode, DEFAULT_ENCODE_SET};

//generic query string builder, automatically escapes each parameter val if necessary
//KEYS ARE TAKEN AS IS

#[derive(Debug)]
pub struct QueryString<'a> {
    params: HashMap<&'a str, String>
}

impl <'a> QueryString<'a> {

    pub fn from (params: Vec<(&'a str, String)>) -> QueryString <'a> {
        let mut hm = QueryString {params: HashMap::new()};
        hm.add_params(params);
        hm
    }

    pub fn add_params (&mut self, params: Vec<(&'a str, String)>) {
        for (key, val) in params {
            self.params.insert(key, val);
        }
    }

    pub fn query_string (&self) -> String {

        let qstring = self.params.iter().map(|(k, v)| k.to_string() + "=" + v)
                          .collect::<Vec<String>>()
                          .join("&");
        utf8_percent_encode(&qstring, DEFAULT_ENCODE_SET)
    }

    pub fn encode_component (component: &[u8]) -> String {
        percent_encode(component, DEFAULT_ENCODE_SET)
    }

}
