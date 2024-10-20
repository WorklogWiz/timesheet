//! This test was written to experiment with dependency injection using the
//! cfg(test) flag
//!
use std::error;
use std::error::Error;

/// Defines the trait for which we will have a mock and a real implementation
trait ComplexService {
    fn do_some_stuff(&self) -> Result<String, Box<dyn error::Error>>;
}

struct MockComplexService {}
impl ComplexService for MockComplexService {
    fn do_some_stuff(&self) -> Result<String, Box<dyn Error>> {
        Ok("Mocked result".into())
    }
}

struct RealComplexService {}
impl ComplexService for RealComplexService {
    fn do_some_stuff(&self) -> Result<String, Box<dyn Error>> {
        Ok("Real service result".into())
    }
}

/// This is the service we will be invoking, into which an implementation of
/// ComplexService has been injected.
struct DummyService<T: ComplexService> {
    service: T,
}

impl <T: ComplexService> DummyService<T> {
    fn invoke_service(&self) -> Result<String, Box<dyn Error>> {
        self.service.do_some_stuff()
    }
}

#[test]
fn test_with_mock() -> Result<(), Box<dyn Error>> {
    let dummy_service = DummyService {
        service: MockComplexService {},
    };
    let s = dummy_service.invoke_service()?;
    assert_eq!(s, "Mocked result");
    Ok(())
}

#[test]
fn test_without_real() -> Result<(), Box<dyn Error>>{
    let dummy_service = DummyService {
        service: RealComplexService{},
    };
    assert_eq!(dummy_service.invoke_service()?, "Real service result");
    Ok(())

}
#[cfg(not(test))]
use RealComplexService as TheService;

#[cfg(test)]
use MockComplexService as TheService;

#[test]
fn test_with_compile_flags() -> Result<(), Box<dyn Error>>{
    let dummy_service = DummyService { service: TheService {} };
    assert_eq!(dummy_service.invoke_service()?, "Mocked result");
    Ok(())
}