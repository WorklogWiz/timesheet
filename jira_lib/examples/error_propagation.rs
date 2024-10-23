use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use crate::MiddleEarth::NoService;
use crate::UnderGround::HttpNotResponding;

#[derive(Debug)]
enum MiddleEarth {
    NoService {source: UnderGround },
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
        match self { NoService { source } => { write!(f,"no service: {}", source)} }
    }
}

impl std::error::Error for MiddleEarth {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            NoService { source } => { Some(source) }
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
            UnderGround::HttpNotResponding(_) => { write!(f, "HTTP server not responding")}
        }
    }
}

impl std::error::Error for UnderGround {

}

fn main() {

    let x = middle_earth();
    match x {
        Ok(_) => {}
        Err(e) => {
            println!("{}", e);
        }
    }
}

fn middle_earth() -> Result<(), MiddleEarth> {
    let _result = underground().map_err(|e| MiddleEarth::from(e) )?;
    Ok(())
}

fn underground() -> Result<(), UnderGround> {
    Err(UnderGround::HttpNotResponding("http://fakeserver".into()))
}
