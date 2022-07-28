use mimicry::*;
use std::str::FromStr;
use strum_macros::{Display, EnumIter};

////////////////////// Data model ////////////////////////
#[derive(PartialEq, Debug, Clone, EnumIter, Display, Default, Mimic)]
pub enum RequestAction {
    #[default]
    None,
    Connect {
        name:           String,
        client_version: String,
    },
    Disconnect,
    KeepAlive {
        latest_response_ack: u64,
    },
    ListPlayers,
    ChatMessage {
        message: String,
    },
    ListRooms,
    NewRoom {
        room_name: String,
    },
    JoinRoom {
        room_name: String,
    },
    LeaveRoom,
    SetClientOptions {
        key:   String,
        //PR_GATE address why this panics Mimicry
        //value: Cli1entOptionValueStruct,
    },
    DropPattern {
        x:       i32,
        y:       i32,
        pattern: String,
    },
    ClearArea {
        x: i32,
        y: i32,
        w: u32,
        h: u32,
    },

}

/*
#[derive(PartialEq, Debug, Clone, Default, Mimic)]
pub struct ClientOptionValueStruct {
    opt_value: Option<ClientOptionValue>,
}

#[derive(PartialEq, Debug, Clone)]
pub enum ClientOptionValue {
    Bool { value: bool },
    U8 { value: u8 },
    U16 { value: u16 },
    U32 { value: u32 },
    U64 { value: u64 },
    I8 { value: i8 },
    I16 { value: i16 },
    I32 { value: i32 },
    I64 { value: i64 },
    Str { value: String },
}

impl From<&str> for ClientOptionValueStruct {
    fn from(s: &str) -> Self {
        let s = s.trim();

        // Adequate for this example
        if s.to_ascii_lowercase() == "true" {
            ClientOptionValueStruct {
                opt_value: Some(ClientOptionValue::Bool { value: true }),
            }
        } else {
            ClientOptionValueStruct {
                opt_value: Some(ClientOptionValue::Bool { value: false }),
            }
        }
    }
}

impl FromStr for ClientOptionValueStruct {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let a;

        if let Ok(value) = s.parse::<bool>() {
            a = Ok(ClientOptionValueStruct {
                opt_value: Some(ClientOptionValue::Bool { value }),
            });
        }
        else
        {
            let err = format!("ClientOptionValueStruct failed parse on: {:?}",s);
            a = Err(err.as_str());
        }
        a
    }
}
*/
