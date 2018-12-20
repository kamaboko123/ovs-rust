extern crate serde_json;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OvsPort{
    pub name : String,
    pub uuid : String,
    pub mode : OvsPortMode
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum OvsPortMode{
    Access(u16),
    Trunk(Vec<serde_json::Value>)
}

impl OvsPort{
    pub fn new(name:&str, uuid:&str, mode:&OvsPortMode) -> OvsPort{
        OvsPort{
            name: name.to_string(),
            uuid : uuid.to_string(),
            mode : mode.clone()
        }
    }
}

