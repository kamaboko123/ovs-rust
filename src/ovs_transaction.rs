use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum PortReqParam{
    String(String),
    OvsPortInsert(OvsPortInsert)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OvsPortInsert{
    #[serde(rename="name-uuid")]
    pub uuid: String,
    pub op: String,
    pub table: String
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PortReq{
    pub method : String,
    pub params : Vec<PortReqParam>
}

impl OvsPortInsert{
    pub fn new()->OvsPortInsert{
        let tmp_uuid = Uuid::new_v4();
        OvsPortInsert{
            uuid:tmp_uuid.to_string(),
            op:"insert".to_string(),
            table:"Port".to_string()
        }
    }
}

