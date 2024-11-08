use common::config;
use common::config::ApplicationConfig;

pub trait ApplicationRuntime {
    fn get_application_configuration(&self) -> config::ApplicationConfig;
    // fn getLocalWroklogDataStore()
}
pub struct ApplicationProductionRuntime {

}

impl ApplicationRuntime for ApplicationProductionRuntime {


    fn get_application_configuration(&self) -> ApplicationConfig {
        todo!()
    }
}

impl ApplicationProductionRuntime {
    fn new() -> Box<dyn ApplicationRuntime> {
        Box::new(ApplicationProductionRuntime {})
    }

}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_application_runtime() {
        let runtime = ApplicationProductionRuntime::new();
        let application_config = runtime.get_application_configuration();


    }
}
