use greetd_ipc::Request;

pub trait SafeDebug {
    fn safe_repr(&self) -> String;
}

impl SafeDebug for Request {
    fn safe_repr(&self) -> String {
        match self {
            msg @ &Request::CancelSession => format!("{:?}", msg),
            msg @ &Request::CreateSession { .. } => format!("{:?}", msg),
            &Request::PostAuthMessageResponse { .. } => "PostAuthMessageResponse".to_string(),
            msg @ &Request::StartSession { .. } => format!("{:?}", msg),
        }
    }
}
