extern crate serde;
extern crate serde_json;

use std::net::TcpStream;
use std::net::Shutdown;
use std::io::Write;
use std::io::Read;
use std::string::FromUtf8Error;
use std::error::Error;
use std::fmt;
use std::fmt::Display;

pub struct OvsClient{
    transaction_id : i32,
    target : String
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OvsPort{
    pub name : String,
    pub uuid : String,
    pub mode : OvsPortMode
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OvsBridge{
    pub name : String,
    pub uuid : String,
    pub ports : Vec<OvsPort>
}

#[derive(Debug)]
enum OvsErrorType{
    ConnectionError,
    InvalidResponse,
    InvalidResponseJson,
    QueryError,
    UnexpectedResponse,
    InconsistentInstruction
}

#[derive(Debug)]
pub struct OvsError{
    error_type: OvsErrorType,
    error_message: String,
    error_detail: String
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum OvsPortMode{
    Access(u16),
    Trunk(Vec<serde_json::Value>)
}

impl OvsError{
    fn new(t: OvsErrorType, message: &str) -> OvsError{
        OvsError{
            error_type: t,
            error_message : message.to_string(),
            error_detail : "".to_string()
        }
    }
    
    fn detail(mut self, detail: &str) -> Self{
        self.error_detail = detail.to_string();
        self
    }
}

impl OvsPort{
    fn new(name:&str, uuid:&str, mode:&OvsPortMode) -> OvsPort{
        OvsPort{
            name: name.to_string(),
            uuid : uuid.to_string(),
            mode : mode.clone()
        }
    }
}

impl OvsBridge{
    fn new(name:&str, uuid:&str) -> OvsBridge{
        OvsBridge{
            name: name.to_string(),
            uuid : uuid.to_string(),
            ports : Vec::new()
        }
    }
}
fn u8v_to_string(v : Vec<u8>) -> Result<String, FromUtf8Error>{
    String::from_utf8(v)
}

impl Display for OvsError{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result{
        if(self.error_detail == ""){
            write!(f, "[OvsError]{}", self.error_message)
        }
        else{
            write!(f, "[OvsError]{}\n  ->(detail){}", self.error_message, self.error_detail)
        }
    }
}

impl Error for OvsError{
    fn description(&self) -> &str{
        "OvsError"
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
enum PortReqParam{
    String(String),
    OvsPort(OvsPort)
}

#[derive(Serialize, Deserialize, Debug)]
struct PortReq{
    method : String,
    params : Vec<PortReqParam>
}

/*
impl Display for PortReq{
    fn fmt(&self, f:&mut fmt::Formatter) -> fmt::Result{
        write!(f, "[OvsError]{}\n  ->{}", self.error_message, self.error_detail)
    }
}*/

impl OvsClient{
    pub fn new(host: &str, port:i16) -> Result<OvsClient, OvsError>{
        Ok(OvsClient{
            transaction_id : 0,
            target : format!("{}:{}", host, port)
        })
    }
    
    fn _check_connection(&mut self) -> bool{
        let query = serde_json::from_str(
            "{\"method\": \"transact\",\"params\":[\"Open_vSwitch\",{\"op\":\"select\",\"table\":\"Port\",\"where\":[]}],\"id\":0}"
        ).unwrap();
        
        match self._send(query){
            Ok(_) => true,
            Err(_) => false
        }
    }
    
    pub fn get_ports(&mut self) -> Result<Vec<OvsPort>, OvsError>{
        let query = serde_json::from_str(
            "{\"method\": \"transact\",\"params\":[\"Open_vSwitch\",{\"op\":\"select\",\"table\":\"Port\",\"where\":[]}],\"id\":0}"
        ).unwrap();
        
        let resp = self._send(query);
        let mut ports:Vec<OvsPort> = Vec::new();
        
        match resp{
            Err(e) => return Err(e),
            Ok(data) => {
                for p in data["result"][0]["rows"].as_array().unwrap(){
                    
                    let name: &str= p["name"].as_str().ok_or(
                        OvsError::new(
                            OvsErrorType::UnexpectedResponse,
                            "key ['name'] is not found in response data"
                        )
                    )?;
                    
                    let uuid: &str= p["_uuid"][1].as_str().ok_or(
                        OvsError::new(
                            OvsErrorType::UnexpectedResponse,
                            "key [_'uuid'][1] is not found in response data"
                        )
                    )?;
                    
                    if p["trunks"][1].as_array().unwrap().len() > 0{
                        ports.push(OvsPort::new(
                            name,
                            uuid,
                            &OvsPortMode::Trunk(p["trunks"][1].as_array().unwrap().clone())
                        ));
                        continue;
                    }
                    
                    match p["tag"].as_u64(){
                        None => {},
                        Some(vlan) => {
                            ports.push(OvsPort::new(
                                name,
                                uuid,
                                &OvsPortMode::Access(vlan as u16)
                            ));
                            continue;
                        }
                    }
                    
                    ports.push(OvsPort::new(
                        name,
                        uuid,
                        &OvsPortMode::Trunk(Vec::new())
                    ));
                    
                }
            }
        }
        
        Ok(ports)
    }
    
    pub fn get_bridges(&mut self) -> Result<Vec<OvsBridge>, OvsError>{
        let query = serde_json::from_str(
            "{\"method\": \"transact\",\"params\":[\"Open_vSwitch\",{\"op\":\"select\",\"table\":\"Bridge\",\"where\":[]}],\"id\":0}"
        ).unwrap();
        
        let resp = self._send(query);
        let mut bridges:Vec<OvsBridge> = Vec::new();
        
        match resp{
            Err(e) => return Err(e),
            Ok(data) => {
                let ports = self.get_ports()?;
                
                for br in data["result"][0]["rows"].as_array().unwrap(){
                    let name: &str= br["name"].as_str().ok_or(
                        OvsError::new(
                            OvsErrorType::UnexpectedResponse,
                            "key ['name'] is not found in response data"
                        )
                    )?;
                    
                    let uuid: &str= br["_uuid"][1].as_str().ok_or(
                        OvsError::new(
                            OvsErrorType::UnexpectedResponse,
                            "key [_'uuid'][1] is not found in response data"
                        )
                    )?;
                    
                    let mut b = OvsBridge::new(
                        name,
                        uuid
                    );
                    
                    match br["ports"][1].as_array(){
                        None=>{},
                        Some(br_ports)=>{
                            for bp in br_ports{
                                for p in &ports{
                                    if bp[1]== p.uuid{
                                        b.ports.push(p.clone());
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    
                    bridges.push(b);
                }
            }
        }
        
        Ok(bridges)
    }
    
    pub fn add_port(&mut self, bridge_name:&str, port_name: &str, port_mode: &OvsPortMode) -> Result<u8, OvsError>{
        /*
        let base: serde_json::Value = serde_json::from_str(r#"
            {"test":"abc"}
        "#).unwrap();
        
        */
        
        let mut ports = self.get_ports()?;
        let mut bridges = self.get_bridges()?;
        
        for p in ports{
            if(p.name == port_name){
                return Err(
                        OvsError::new(
                        OvsErrorType::InconsistentInstruction,
                        &format!("Interface already exist in ovsdb. ({})", port_name)
                    )
                )
            }
        }
        
        let mut target_bridge: Option<&OvsBridge> = None;
        
        for i in  0..(bridges.len() - 1){
            if(bridges[i].name == bridge_name){
                target_bridge = Some(&bridges[i]);
            }
        }
        
        match target_bridge{
            None=>{
                return Err(
                        OvsError::new(
                        OvsErrorType::InconsistentInstruction,
                        &format!("Bridge is not found. ({})", bridge_name)
                    )
                )
            },
            Some(b) =>{
                println!("{}", serde_json::to_string(b).unwrap());
            }
        }
        
        let mut base = PortReq{
            method : "transact".to_string(),
            params : Vec::new()
        };
        
        
        
        //enp3s0
        base.params.push(PortReqParam::String("Open_vSwitch".to_string()));
        base.params.push(PortReqParam::OvsPort(OvsPort::new(port_name, "enp3s0", port_mode)));
        
        println!("{}", serde_json::to_string(&base).unwrap());
        
        Ok(1)
    }
    
    fn _send(&mut self, msg : serde_json::Value) -> Result<serde_json::Value, OvsError>{
        self.transaction_id += 1;
        let mut socket = match TcpStream::connect(&self.target){
            Ok(con) => con,
            Err(e) =>return  Err(
                OvsError::new(
                    OvsErrorType::ConnectionError,
                    &format!("failed to connect ovs ({})", self.target)
                )
                .detail(&e.to_string())
            )
        };
        
        
        socket.write(msg.to_string().as_bytes())
        .map_err(
            |e| OvsError::new(OvsErrorType::ConnectionError, "Faild to send request data").detail(&e.to_string())
        )?;
        
        let _ = socket.flush();
        let _ = socket.shutdown(Shutdown::Write);
        
        let mut s : Vec<u8> = Vec::new();
        
        let resp_str = try!(
            socket.read_to_end(&mut s)
            .map_err(
                |_| OvsError::new(OvsErrorType::InvalidResponse, "Failed to read response data")
            )
            .and_then(
                |_| u8v_to_string(s)
                .map_err(
                    |_| OvsError::new(OvsErrorType::InvalidResponse, "Failed to read response data")
                )
            )
        );
        
        let resp_json: serde_json::Value = try!(
            serde_json::from_str(resp_str.as_str())
            .map_err(
                |_| OvsError::new(OvsErrorType::InvalidResponseJson, "Faild to parse response data")
            )
        );
        
        match resp_json["result"][0].as_object(){
            None => {
                return Err(OvsError::new(OvsErrorType::InvalidResponse, "Faild to parse response data"))
            },
            Some(result) => {
                if result.contains_key("error"){
                    return Err(
                        OvsError::new(OvsErrorType::QueryError, "Client received error response. Please check detail.")
                        .detail(&resp_json["result"][0].to_string())
                    )
                }
            }
        }
        
        Ok(resp_json)
    }
}

