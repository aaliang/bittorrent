use std::collections::HashMap;
use url::percent_encoding::{utf8_percent_encode, DEFAULT_ENCODE_SET};

//generic query string builder, automatically escapes each parameter val if necessary
//KEYS ARE TAKEN AS IS

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
            self.params.insert(key, utf8_percent_encode(&val, DEFAULT_ENCODE_SET));
        }
    }

    pub fn to_param_string (&self) -> String {
        self.params.iter().map(|(k, v)| k.to_string() + "=" + v)
                          .collect::<Vec<String>>()
                          .join("&")
    }

}
