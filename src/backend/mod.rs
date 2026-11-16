pub mod c;
pub mod js;

#[derive(Debug, Clone, Copy)]
pub enum Backend { C, Js }

impl Backend {
    pub fn from_str(s: &str) -> Option<Backend> {
        match s {
            "c" => Some(Backend::C),
            "js" => Some(Backend::Js),
            _ => None,
        }
    }
}