//! ComputeBrick: WebGPU shader generation from brick definitions (PROBAR-SPEC-009-P8)
//!
//! Generates WGSL shaders and wgpu bindings from a single brick definition.
//! Zero hand-written shaders - all code derived from Rust types.
//!
//! # Design Philosophy
//!
//! ComputeBrick applies the same zero-artifact principle to GPU compute:
//! - Define tensor shapes and operations in Rust
//! - Generate WGSL shader code
//! - Generate Rust wgpu bindings
//!
//! # Inspiration
//!
//! NVIDIA CUDA Tile IR provides the model for declarative GPU programming.
//! ComputeBrick adapts these patterns for WebGPU.
//!
//! # Example
//!
//! ```rust,ignore
//! use probar::brick::compute::{ComputeBrick, TensorBinding, TensorType, TileStrategy, TileOp};
//!
//! let mel_brick = ComputeBrick::new("mel-filterbank")
//!     .workgroup_size(256, 1, 1)
//!     .input("audio", TensorType::F32, &[CHUNK_SIZE])
//!     .output("mel", TensorType::F32, &[N_MELS, N_FRAMES])
//!     .tile_strategy(TileStrategy::Simple2D { tile_x: 16, tile_y: 16 })
//!     .op(TileOp::LoadShared { src: "audio".into(), tile_size: (256, 1) })
//!     .op(TileOp::Elementwise { op: ElementwiseOp::Log, operands: vec!["audio".into()] })
//!     .op(TileOp::StoreShared { dst: "mel".into() });
//!
//! // Generate WGSL
//! let wgsl = mel_brick.to_wgsl();
//! ```

use super::{Brick, BrickAssertion, BrickBudget, BrickVerification};
use std::time::Duration;

/// Tensor element type for GPU compute
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TensorType {
    /// 32-bit float
    F32,
    /// 16-bit float (half precision)
    F16,
    /// 32-bit signed integer
    I32,
    /// 32-bit unsigned integer
    U32,
}

impl TensorType {
    /// Get WGSL type name
    #[must_use]
    pub fn to_wgsl(&self) -> &'static str {
        match self {
            Self::F32 => "f32",
            Self::F16 => "f16",
            Self::I32 => "i32",
            Self::U32 => "u32",
        }
    }

    /// Get Rust type name
    #[must_use]
    pub fn to_rust(&self) -> &'static str {
        match self {
            Self::F32 => "f32",
            Self::F16 => "half::f16",
            Self::I32 => "i32",
            Self::U32 => "u32",
        }
    }

    /// Get byte size
    #[must_use]
    pub const fn byte_size(&self) -> usize {
        match self {
            Self::F32 | Self::I32 | Self::U32 => 4,
            Self::F16 => 2,
        }
    }
}

/// A tensor binding for compute shader
#[derive(Debug, Clone)]
pub struct TensorBinding {
    /// Binding name
    pub name: String,
    /// Element type
    pub dtype: TensorType,
    /// Shape dimensions
    pub shape: Vec<u32>,
    /// Binding group
    pub group: u32,
    /// Binding index within group
    pub binding: u32,
    /// Read-only flag
    pub read_only: bool,
}

impl TensorBinding {
    /// Create a new tensor binding
    #[must_use]
    pub fn new(name: impl Into<String>, dtype: TensorType, shape: &[u32]) -> Self {
        Self {
            name: name.into(),
            dtype,
            shape: shape.to_vec(),
            group: 0,
            binding: 0,
            read_only: true,
        }
    }

    /// Set binding group and index
    #[must_use]
    pub fn at(mut self, group: u32, binding: u32) -> Self {
        self.group = group;
        self.binding = binding;
        self
    }

    /// Mark as writable
    #[must_use]
    pub fn writable(mut self) -> Self {
        self.read_only = false;
        self
    }

    /// Get total element count
    #[must_use]
    pub fn element_count(&self) -> u32 {
        self.shape.iter().product()
    }

    /// Get total byte size
    #[must_use]
    pub fn byte_size(&self) -> usize {
        self.element_count() as usize * self.dtype.byte_size()
    }

    /// Generate WGSL binding declaration
    #[must_use]
    pub fn to_wgsl_binding(&self) -> String {
        let access = if self.read_only { "read" } else { "read_write" };
        format!(
            "@group({}) @binding({}) var<storage, {}> {}: array<{}>;",
            self.group,
            self.binding,
            access,
            self.name,
            self.dtype.to_wgsl()
        )
    }
}

/// Tiling strategy for GPU compute
#[derive(Debug, Clone)]
pub enum TileStrategy {
    /// Simple 2D tiling
    Simple2D {
        /// Tile width
        tile_x: u32,
        /// Tile height
        tile_y: u32,
    },
    /// Cooperative matrix (tensor core style)
    Cooperative {
        /// Matrix M dimension
        m: u32,
        /// Matrix N dimension
        n: u32,
        /// Matrix K dimension
        k: u32,
    },
    /// Streaming (for convolutions)
    Streaming {
        /// Window size
        window: u32,
    },
    /// No tiling (direct compute)
    None,
}

impl TileStrategy {
    /// Get optimal workgroup size for this strategy
    #[must_use]
    pub fn optimal_workgroup_size(&self) -> (u32, u32, u32) {
        match self {
            Self::Simple2D { tile_x, tile_y } => (*tile_x, *tile_y, 1),
            Self::Cooperative { m, n, .. } => (*m, *n, 1),
            Self::Streaming { window } => (*window, 1, 1),
            Self::None => (64, 1, 1),
        }
    }
}

/// Element-wise operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementwiseOp {
    /// Natural logarithm
    Log,
    /// Exponential
    Exp,
    /// Square root
    Sqrt,
    /// Absolute value
    Abs,
    /// Rectified linear unit
    Relu,
    /// Sigmoid
    Sigmoid,
    /// Hyperbolic tangent
    Tanh,
    /// Add constant
    AddScalar(i32),
    /// Multiply by constant
    MulScalar(i32),
    /// Clamp to range
    Clamp,
}

impl ElementwiseOp {
    /// Generate WGSL expression for this operation
    #[must_use]
    pub fn to_wgsl_expr(&self, operand: &str) -> String {
        match self {
            Self::Log => format!("log({})", operand),
            Self::Exp => format!("exp({})", operand),
            Self::Sqrt => format!("sqrt({})", operand),
            Self::Abs => format!("abs({})", operand),
            Self::Relu => format!("max({}, 0.0)", operand),
            Self::Sigmoid => format!("1.0 / (1.0 + exp(-{}))", operand),
            Self::Tanh => format!("tanh({})", operand),
            Self::AddScalar(s) => format!("({} + {}.0)", operand, s),
            Self::MulScalar(s) => format!("({} * {}.0)", operand, s),
            Self::Clamp => format!("clamp({}, 0.0, 1.0)", operand),
        }
    }
}

/// Tile operation in compute shader
#[derive(Debug, Clone)]
pub enum TileOp {
    /// Load tile from global to shared memory
    LoadShared {
        /// Source tensor name
        src: String,
        /// Tile dimensions
        tile_size: (u32, u32),
    },
    /// Matrix multiply accumulate (tensor core pattern)
    Mma {
        /// Input A tensor
        a: String,
        /// Input B tensor
        b: String,
        /// Output C tensor
        c: String,
    },
    /// Element-wise operation
    Elementwise {
        /// Operation type
        op: ElementwiseOp,
        /// Input operand names
        operands: Vec<String>,
        /// Output name (defaults to first operand if None)
        output: Option<String>,
    },
    /// Store tile from shared to global memory
    StoreShared {
        /// Destination tensor name
        dst: String,
    },
    /// Synchronization barrier
    Barrier,
    /// Reduction operation (sum, max, min)
    Reduce {
        /// Reduction type
        kind: ReduceKind,
        /// Input tensor
        input: String,
        /// Output scalar or reduced tensor
        output: String,
    },
}

/// Reduction operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReduceKind {
    /// Sum all elements
    Sum,
    /// Maximum element
    Max,
    /// Minimum element
    Min,
    /// Mean of elements
    Mean,
}

impl ReduceKind {
    /// Get WGSL identity value
    #[must_use]
    pub fn identity(&self) -> &'static str {
        match self {
            Self::Sum | Self::Mean => "0.0",
            Self::Max => "-3.402823e+38", // f32::MIN
            Self::Min => "3.402823e+38",  // f32::MAX
        }
    }

    /// Get WGSL combine operation
    #[must_use]
    pub fn combine_op(&self) -> &'static str {
        match self {
            Self::Sum | Self::Mean => "+",
            Self::Max => "max",
            Self::Min => "min",
        }
    }
}

/// ComputeBrick: Generates WebGPU shaders from brick definition
#[derive(Debug, Clone)]
pub struct ComputeBrick {
    /// Shader name
    name: String,
    /// Workgroup size
    workgroup_size: (u32, u32, u32),
    /// Input tensor bindings
    inputs: Vec<TensorBinding>,
    /// Output tensor bindings
    outputs: Vec<TensorBinding>,
    /// Tiling strategy
    tile_strategy: TileStrategy,
    /// Operations to perform
    operations: Vec<TileOp>,
    /// Shared memory allocations
    shared_memory: Vec<(String, TensorType, u32)>,
}

impl ComputeBrick {
    /// Create a new compute brick
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            workgroup_size: (64, 1, 1),
            inputs: Vec::new(),
            outputs: Vec::new(),
            tile_strategy: TileStrategy::None,
            operations: Vec::new(),
            shared_memory: Vec::new(),
        }
    }

    /// Set workgroup size
    #[must_use]
    pub fn workgroup_size(mut self, x: u32, y: u32, z: u32) -> Self {
        self.workgroup_size = (x, y, z);
        self
    }

    /// Add an input tensor
    #[must_use]
    pub fn input(mut self, name: impl Into<String>, dtype: TensorType, shape: &[u32]) -> Self {
        let binding_idx = self.inputs.len() as u32;
        self.inputs
            .push(TensorBinding::new(name, dtype, shape).at(0, binding_idx));
        self
    }

    /// Add an output tensor
    #[must_use]
    pub fn output(mut self, name: impl Into<String>, dtype: TensorType, shape: &[u32]) -> Self {
        let binding_idx = self.outputs.len() as u32;
        self.outputs.push(
            TensorBinding::new(name, dtype, shape)
                .at(1, binding_idx)
                .writable(),
        );
        self
    }

    /// Set the tiling strategy
    #[must_use]
    pub fn tile_strategy(mut self, strategy: TileStrategy) -> Self {
        self.tile_strategy = strategy;
        self
    }

    /// Add an operation
    #[must_use]
    pub fn op(mut self, operation: TileOp) -> Self {
        self.operations.push(operation);
        self
    }

    /// Allocate shared memory
    #[must_use]
    pub fn shared(mut self, name: impl Into<String>, dtype: TensorType, size: u32) -> Self {
        self.shared_memory.push((name.into(), dtype, size));
        self
    }

    /// Generate WGSL shader code
    #[must_use]
    pub fn to_wgsl(&self) -> String {
        let mut wgsl = String::new();

        // Header comment
        wgsl.push_str(&format!(
            "// {} Compute Shader\n",
            to_pascal_case(&self.name)
        ));
        wgsl.push_str("// Generated by probar ComputeBrick - DO NOT EDIT MANUALLY\n\n");

        // Input bindings
        for input in &self.inputs {
            wgsl.push_str(&input.to_wgsl_binding());
            wgsl.push('\n');
        }

        // Output bindings
        for output in &self.outputs {
            wgsl.push_str(&output.to_wgsl_binding());
            wgsl.push('\n');
        }

        wgsl.push('\n');

        // Shared memory declarations
        for (name, dtype, size) in &self.shared_memory {
            wgsl.push_str(&format!(
                "var<workgroup> {}: array<{}, {}>;\n",
                name,
                dtype.to_wgsl(),
                size
            ));
        }

        if !self.shared_memory.is_empty() {
            wgsl.push('\n');
        }

        // Main compute function
        let (wg_x, wg_y, wg_z) = self.workgroup_size;
        wgsl.push_str(&format!(
            "@compute @workgroup_size({}, {}, {})\n",
            wg_x, wg_y, wg_z
        ));
        wgsl.push_str("fn main(\n");
        wgsl.push_str("    @builtin(global_invocation_id) global_id: vec3<u32>,\n");
        wgsl.push_str("    @builtin(local_invocation_id) local_id: vec3<u32>,\n");
        wgsl.push_str("    @builtin(workgroup_id) workgroup_id: vec3<u32>,\n");
        wgsl.push_str(") {\n");

        // Index calculations
        wgsl.push_str("    let gid = global_id.x + global_id.y * ");
        wgsl.push_str(&format!("{}u;\n", wg_x));
        wgsl.push_str("    let lid = local_id.x + local_id.y * ");
        wgsl.push_str(&format!("{}u;\n\n", wg_x));

        // Generate operations
        for op in &self.operations {
            match op {
                TileOp::LoadShared { src, tile_size: _ } => {
                    wgsl.push_str(&format!("    // Load from {} to shared memory\n", src));
                    wgsl.push_str(&format!("    let val_{} = {}[gid];\n", src, src));
                }
                TileOp::Elementwise {
                    op: elem_op,
                    operands,
                    output,
                } => {
                    let input = &operands[0];
                    let out_name = output.as_ref().unwrap_or(input);
                    let input_val = format!("val_{}", input);
                    let expr = elem_op.to_wgsl_expr(&input_val);
                    wgsl.push_str(&format!("    let val_{} = {};\n", out_name, expr));
                }
                TileOp::StoreShared { dst } => {
                    wgsl.push_str(&format!("    // Store to {}\n", dst));
                    // Find what value to store
                    let val_name = if self.operations.iter().any(
                        |o| matches!(o, TileOp::Elementwise { output: Some(n), .. } if n == dst),
                    ) {
                        format!("val_{}", dst)
                    } else if let Some(input) = self.inputs.first() {
                        format!("val_{}", input.name)
                    } else {
                        "0.0".to_string()
                    };
                    wgsl.push_str(&format!("    {}[gid] = {};\n", dst, val_name));
                }
                TileOp::Barrier => {
                    wgsl.push_str("    workgroupBarrier();\n");
                }
                TileOp::Mma { a, b, c } => {
                    wgsl.push_str(&format!("    // Matrix multiply: {} = {} @ {}\n", c, a, b));
                    wgsl.push_str(&format!("    // TODO: Implement cooperative matrix\n"));
                }
                TileOp::Reduce {
                    kind,
                    input,
                    output,
                } => {
                    wgsl.push_str(&format!(
                        "    // Reduce {} -> {} ({:?})\n",
                        input, output, kind
                    ));
                }
            }
        }

        wgsl.push_str("}\n");

        wgsl
    }

    /// Generate Rust wgpu bindings
    #[must_use]
    pub fn to_rust_bindings(&self) -> String {
        let mut rust = String::new();

        // Header
        rust.push_str(&format!(
            "//! {} Compute Bindings\n",
            to_pascal_case(&self.name)
        ));
        rust.push_str("//! Generated by probar ComputeBrick - DO NOT EDIT MANUALLY\n\n");
        rust.push_str(
            "use wgpu::{BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry};\n",
        );
        rust.push_str("use wgpu::{ShaderStages, BufferBindingType, BindingType};\n\n");

        let struct_name = to_pascal_case(&self.name);

        // Struct definition
        rust.push_str(&format!("pub struct {}Compute {{\n", struct_name));
        rust.push_str("    pub pipeline: wgpu::ComputePipeline,\n");
        rust.push_str("    pub bind_group_layout: wgpu::BindGroupLayout,\n");
        rust.push_str("}\n\n");

        // Implementation
        rust.push_str(&format!("impl {}Compute {{\n", struct_name));
        rust.push_str("    pub const WORKGROUP_SIZE: (u32, u32, u32) = ");
        rust.push_str(&format!("{:?};\n\n", self.workgroup_size));

        // WGSL source as const
        rust.push_str("    pub const SHADER_SOURCE: &'static str = r#\"\n");
        rust.push_str(&self.to_wgsl());
        rust.push_str("\"#;\n\n");

        // Create bind group layout
        rust.push_str(
            "    pub fn create_bind_group_layout(device: &wgpu::Device) -> BindGroupLayout {\n",
        );
        rust.push_str("        device.create_bind_group_layout(&BindGroupLayoutDescriptor {\n");
        rust.push_str(&format!(
            "            label: Some(\"{} bind group layout\"),\n",
            self.name
        ));
        rust.push_str("            entries: &[\n");

        for input in &self.inputs {
            rust.push_str(&format!("                // Input: {}\n", input.name));
            rust.push_str(&format!(
                "                BindGroupLayoutEntry {{\n                    binding: {},\n                    visibility: ShaderStages::COMPUTE,\n                    ty: BindingType::Buffer {{\n                        ty: BufferBindingType::Storage {{ read_only: true }},\n                        has_dynamic_offset: false,\n                        min_binding_size: None,\n                    }},\n                    count: None,\n                }},\n",
                input.binding
            ));
        }

        for output in &self.outputs {
            rust.push_str(&format!("                // Output: {}\n", output.name));
            rust.push_str(&format!(
                "                BindGroupLayoutEntry {{\n                    binding: {},\n                    visibility: ShaderStages::COMPUTE,\n                    ty: BindingType::Buffer {{\n                        ty: BufferBindingType::Storage {{ read_only: false }},\n                        has_dynamic_offset: false,\n                        min_binding_size: None,\n                    }},\n                    count: None,\n                }},\n",
                output.binding
            ));
        }

        rust.push_str("            ],\n");
        rust.push_str("        })\n");
        rust.push_str("    }\n");
        rust.push_str("}\n");

        rust
    }

    /// Generate JavaScript dispatch code for WebGPU
    #[must_use]
    pub fn to_dispatch_js(&self) -> String {
        let mut js = String::new();

        js.push_str(&format!(
            "// {} Compute Dispatch\n",
            to_pascal_case(&self.name)
        ));
        js.push_str("// Generated by probar ComputeBrick - DO NOT EDIT MANUALLY\n\n");

        let (wg_x, wg_y, wg_z) = self.workgroup_size;
        js.push_str(&format!(
            "const WORKGROUP_SIZE = [{}, {}, {}];\n\n",
            wg_x, wg_y, wg_z
        ));

        js.push_str(&format!(
            "async function dispatch{}(device, inputs, outputs) {{\n",
            to_pascal_case(&self.name)
        ));

        js.push_str("    // Create shader module\n");
        js.push_str("    const shaderModule = device.createShaderModule({\n");
        js.push_str(&format!("        label: '{} shader',\n", self.name));
        js.push_str("        code: SHADER_SOURCE,\n");
        js.push_str("    });\n\n");

        js.push_str("    // Calculate dispatch size\n");
        if let Some(output) = self.outputs.first() {
            let total_size = output.element_count();
            js.push_str(&format!("    const totalElements = {};\n", total_size));
            js.push_str(&format!(
                "    const numWorkgroups = Math.ceil(totalElements / {});\n\n",
                wg_x * wg_y * wg_z
            ));
        }

        js.push_str("    // Dispatch\n");
        js.push_str("    const commandEncoder = device.createCommandEncoder();\n");
        js.push_str("    const passEncoder = commandEncoder.beginComputePass();\n");
        js.push_str("    passEncoder.setPipeline(pipeline);\n");
        js.push_str("    passEncoder.setBindGroup(0, bindGroup);\n");
        js.push_str("    passEncoder.dispatchWorkgroups(numWorkgroups, 1, 1);\n");
        js.push_str("    passEncoder.end();\n");
        js.push_str("    device.queue.submit([commandEncoder.finish()]);\n");
        js.push_str("}\n");

        js
    }

    /// Get the brick name
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get workgroup size
    #[must_use]
    pub fn get_workgroup_size(&self) -> (u32, u32, u32) {
        self.workgroup_size
    }

    /// Get input bindings
    #[must_use]
    pub fn inputs(&self) -> &[TensorBinding] {
        &self.inputs
    }

    /// Get output bindings
    #[must_use]
    pub fn outputs(&self) -> &[TensorBinding] {
        &self.outputs
    }
}

impl Brick for ComputeBrick {
    fn brick_name(&self) -> &'static str {
        "ComputeBrick"
    }

    fn assertions(&self) -> &[BrickAssertion] {
        &[]
    }

    fn budget(&self) -> BrickBudget {
        // Compute shaders have longer budgets
        BrickBudget::uniform(100)
    }

    fn verify(&self) -> BrickVerification {
        let mut passed = Vec::new();
        let mut failed = Vec::new();

        // Verify workgroup size is valid
        let (x, y, z) = self.workgroup_size;
        if x * y * z > 1024 {
            failed.push((
                BrickAssertion::Custom {
                    name: "workgroup_size_valid".into(),
                    validator_id: 1,
                },
                format!(
                    "Workgroup size {}x{}x{}={} exceeds maximum 1024",
                    x,
                    y,
                    z,
                    x * y * z
                ),
            ));
        } else {
            passed.push(BrickAssertion::Custom {
                name: "workgroup_size_valid".into(),
                validator_id: 1,
            });
        }

        // Verify inputs and outputs are defined
        if self.inputs.is_empty() {
            failed.push((
                BrickAssertion::Custom {
                    name: "has_inputs".into(),
                    validator_id: 2,
                },
                "ComputeBrick has no input tensors".into(),
            ));
        } else {
            passed.push(BrickAssertion::Custom {
                name: "has_inputs".into(),
                validator_id: 2,
            });
        }

        if self.outputs.is_empty() {
            failed.push((
                BrickAssertion::Custom {
                    name: "has_outputs".into(),
                    validator_id: 3,
                },
                "ComputeBrick has no output tensors".into(),
            ));
        } else {
            passed.push(BrickAssertion::Custom {
                name: "has_outputs".into(),
                validator_id: 3,
            });
        }

        // Verify operations reference valid tensors
        let tensor_names: Vec<_> = self
            .inputs
            .iter()
            .chain(self.outputs.iter())
            .map(|t| t.name.as_str())
            .collect();

        for op in &self.operations {
            match op {
                TileOp::LoadShared { src, .. } => {
                    if !tensor_names.contains(&src.as_str()) {
                        failed.push((
                            BrickAssertion::Custom {
                                name: "tensor_exists".into(),
                                validator_id: 4,
                            },
                            format!("LoadShared references unknown tensor: {}", src),
                        ));
                    }
                }
                TileOp::StoreShared { dst } => {
                    if !tensor_names.contains(&dst.as_str()) {
                        failed.push((
                            BrickAssertion::Custom {
                                name: "tensor_exists".into(),
                                validator_id: 4,
                            },
                            format!("StoreShared references unknown tensor: {}", dst),
                        ));
                    }
                }
                _ => {}
            }
        }

        if failed.is_empty() {
            passed.push(BrickAssertion::Custom {
                name: "compute_brick_valid".into(),
                validator_id: 5,
            });
        }

        BrickVerification {
            passed,
            failed,
            verification_time: Duration::from_micros(100),
        }
    }

    fn to_html(&self) -> String {
        // ComputeBrick doesn't generate HTML
        String::new()
    }

    fn to_css(&self) -> String {
        // ComputeBrick doesn't generate CSS
        String::new()
    }
}

/// Convert string to PascalCase
fn to_pascal_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = true;

    for c in s.chars() {
        if c == '_' || c == '-' || c == ' ' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_brick_basic() {
        let brick = ComputeBrick::new("test")
            .workgroup_size(256, 1, 1)
            .input("audio", TensorType::F32, &[1024])
            .output("mel", TensorType::F32, &[80, 100]);

        assert_eq!(brick.name(), "test");
        assert_eq!(brick.get_workgroup_size(), (256, 1, 1));
        assert_eq!(brick.inputs().len(), 1);
        assert_eq!(brick.outputs().len(), 1);
    }

    #[test]
    fn test_compute_brick_wgsl_generation() {
        let brick = ComputeBrick::new("log-transform")
            .workgroup_size(64, 1, 1)
            .input("input", TensorType::F32, &[1024])
            .output("output", TensorType::F32, &[1024])
            .op(TileOp::LoadShared {
                src: "input".into(),
                tile_size: (64, 1),
            })
            .op(TileOp::Elementwise {
                op: ElementwiseOp::Log,
                operands: vec!["input".into()],
                output: Some("output".into()),
            })
            .op(TileOp::StoreShared {
                dst: "output".into(),
            });

        let wgsl = brick.to_wgsl();

        assert!(wgsl.contains("@compute @workgroup_size(64, 1, 1)"));
        assert!(wgsl.contains("fn main("));
        assert!(wgsl.contains("log("));
        assert!(wgsl.contains("Generated by probar"));
    }

    #[test]
    fn test_compute_brick_verification() {
        let brick = ComputeBrick::new("test")
            .workgroup_size(256, 1, 1)
            .input("input", TensorType::F32, &[1024])
            .output("output", TensorType::F32, &[1024]);

        let result = brick.verify();
        assert!(result.is_valid());
    }

    #[test]
    fn test_compute_brick_verification_fails_no_inputs() {
        let brick = ComputeBrick::new("test").workgroup_size(256, 1, 1).output(
            "output",
            TensorType::F32,
            &[1024],
        );

        let result = brick.verify();
        assert!(!result.is_valid());
    }

    #[test]
    fn test_compute_brick_verification_fails_large_workgroup() {
        let brick = ComputeBrick::new("test")
            .workgroup_size(1024, 2, 1) // 2048 > 1024 max
            .input("input", TensorType::F32, &[1024])
            .output("output", TensorType::F32, &[1024]);

        let result = brick.verify();
        assert!(!result.is_valid());
    }

    #[test]
    fn test_tensor_binding() {
        let binding = TensorBinding::new("audio", TensorType::F32, &[1024, 80])
            .at(0, 1)
            .writable();

        assert_eq!(binding.name, "audio");
        assert_eq!(binding.element_count(), 1024 * 80);
        assert_eq!(binding.byte_size(), 1024 * 80 * 4);
        assert!(!binding.read_only);
    }

    #[test]
    fn test_tensor_type_wgsl() {
        assert_eq!(TensorType::F32.to_wgsl(), "f32");
        assert_eq!(TensorType::F16.to_wgsl(), "f16");
        assert_eq!(TensorType::I32.to_wgsl(), "i32");
        assert_eq!(TensorType::U32.to_wgsl(), "u32");
    }

    #[test]
    fn test_elementwise_ops() {
        assert_eq!(ElementwiseOp::Log.to_wgsl_expr("x"), "log(x)");
        assert_eq!(ElementwiseOp::Exp.to_wgsl_expr("x"), "exp(x)");
        assert_eq!(ElementwiseOp::Relu.to_wgsl_expr("x"), "max(x, 0.0)");
        assert_eq!(ElementwiseOp::AddScalar(5).to_wgsl_expr("x"), "(x + 5.0)");
    }

    #[test]
    fn test_rust_bindings_generation() {
        let brick = ComputeBrick::new("mel-transform")
            .workgroup_size(256, 1, 1)
            .input("audio", TensorType::F32, &[1024])
            .output("mel", TensorType::F32, &[80]);

        let rust = brick.to_rust_bindings();

        assert!(rust.contains("pub struct MelTransformCompute"));
        assert!(rust.contains("WORKGROUP_SIZE"));
        assert!(rust.contains("SHADER_SOURCE"));
        assert!(rust.contains("create_bind_group_layout"));
    }

    #[test]
    fn test_js_dispatch_generation() {
        let brick = ComputeBrick::new("fft")
            .workgroup_size(64, 1, 1)
            .input("signal", TensorType::F32, &[512])
            .output("spectrum", TensorType::F32, &[512]);

        let js = brick.to_dispatch_js();

        assert!(js.contains("async function dispatchFft"));
        assert!(js.contains("WORKGROUP_SIZE"));
        assert!(js.contains("dispatchWorkgroups"));
    }

    #[test]
    fn test_tile_strategy_workgroup_size() {
        let simple = TileStrategy::Simple2D {
            tile_x: 16,
            tile_y: 16,
        };
        assert_eq!(simple.optimal_workgroup_size(), (16, 16, 1));

        let coop = TileStrategy::Cooperative { m: 8, n: 8, k: 4 };
        assert_eq!(coop.optimal_workgroup_size(), (8, 8, 1));

        let streaming = TileStrategy::Streaming { window: 32 };
        assert_eq!(streaming.optimal_workgroup_size(), (32, 1, 1));
    }
}
