use crate::MiddleEarth::NoService;
use crate::UnderGround::HttpNotResponding;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};

#[derive(Debug)]
enum MiddleEarth {
    NoService { source: UnderGround },
}

impl From<UnderGround> for MiddleEarth {
    fn from(value: UnderGround) -> Self {
        match &value {
            HttpNotResponding(_s) => NoService { source: value },
        }
    }
}

impl Display for MiddleEarth {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            NoService { source } => {
                write!(f, "no service: {source}")
            }
        }
    }
}

impl std::error::Error for MiddleEarth {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            NoService { source } => Some(source),
        }
    }
}

#[derive(Debug)]
enum UnderGround {
    HttpNotResponding(String),
}

impl Display for UnderGround {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            UnderGround::HttpNotResponding(server) => {
                write!(f, "HTTP server at {} not responding", server)
            }
        }
    }
}

impl std::error::Error for UnderGround {}

fn main() {
    let x = middle_earth();
    match x {
        Ok(()) => {}
        Err(e) => {
            println!("{e}");
        }
    }
}

fn middle_earth() -> Result<(), MiddleEarth> {
    underground().map_err(MiddleEarth::from)?; // TODO: what if I need more context here?
    Ok(())
}

fn underground() -> Result<(), UnderGround> {
    Err(UnderGround::HttpNotResponding("http://fakeserver".into()))
}
