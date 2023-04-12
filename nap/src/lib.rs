use std::marker::PhantomData;

use nu_plugin::{EvaluatedCall, Plugin as NuPlugin, LabeledError, MsgPackSerializer};
use nu_protocol::{PluginSignature as NuPluginSignature, Value};

pub trait PluginSignatures: Sized {
    fn signature() -> Vec<NuPluginSignature>;
    fn parse_call(name: &str, call: &EvaluatedCall) -> Result<Self, LabeledError>;
}

struct Plugin<'a, T: PluginSignatures, F: FnMut(T, &Value) -> Result<Value, LabeledError>> {
    data: PhantomData<T>,
    main: &'a mut F
}

impl <'a, T: PluginSignatures, F: FnMut(T, &Value) -> Result<Value, LabeledError>> NuPlugin for Plugin<'a, T, F> {
    fn signature(&self) -> Vec<NuPluginSignature> {
        T::signature()
    }

    fn run(
        &mut self,
        name: &str,
        call: &EvaluatedCall,
        input: &nu_protocol::Value,
    ) -> Result<nu_protocol::Value, nu_plugin::LabeledError> {
        let data = T::parse_call(name, call)?;
        (self.main)(data, input)
    }
}

pub fn serve_plugin<T: PluginSignatures, F: FnMut(T, &Value) -> Result<Value, LabeledError>>(main: &mut F) {
    let mut plugin = Plugin {
        data: PhantomData,
        main: main
    };
    nu_plugin::serve_plugin(&mut plugin, MsgPackSerializer {})
}
