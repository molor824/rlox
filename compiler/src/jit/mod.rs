use cranelift::frontend::FunctionBuilderContext;
use cranelift_jit::JITModule;
use cranelift_module::DataDescription;

pub struct JIT {
    builder_ctx: FunctionBuilderContext,
    ctx: cranelift::codegen::Context,
    data_desc: DataDescription,
    module: JITModule,
}
