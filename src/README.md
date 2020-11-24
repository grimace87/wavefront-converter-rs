
### Model converter utility for Vulkano

The purpose of this tool is to pre-process Wavefront (`*.obj`) model files, which are text-based
and order data in a way that is not immediately convenient for computer graphics, and generate
binary files containing the data for the source models that will be faster to read at run time.
In addition to avoiding text parsing at run time, the data will be structured in a way that is
close to how model data can be typically loaded into a graphics API (such as OpenGL or Vulkan)
using vertex buffers and index buffers.

This tool is intended for use with [Vulkano](https://github.com/vulkano-rs/vulkano), and may have
no utility otherwise. It is not an official part of Vulkano and has no affiliation with the
Vulkano developers. 

#### Usage

Include this tool in `Cargo.toml` as a dependency for both the build script and source code:

```toml
[dependencies]
# Other dependencies...
wavefront-converter-rs = { git = "https://github.com/grimace87/wavefront-converter-rs.git" }

[build-dependencies]
wavefront-converter-rs = { git = "https://github.com/grimace87/wavefront-converter-rs.git" }
```

Then, add the tool as a step in `build.rs`, supplying the directory containing the Wavefront
models as well as an arbitrary output directory name:

```rust
use std::env;
use std::path::PathBuf;

fn main() {
    let mut src_dir = std::env::current_dir().unwrap();
    dir.push("models_src_dir");
    let mut dst_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    dst_dir.push("models");
    if !dst_dir.is_dir() {
        std::fs::create_dir(&dst_dir).unwrap();
    }
    wavefront_converter_rs::process_directory(&src_dir, &dst_dir);
}
```

Now, the contents of generated files can be included into the compilation unit and efficiently
read during run time:

```rust
use wavefront_converter_rs::model::{Model, Vertex};

// File names will match the name of the models in the source .obj file
const SOME_MODEL_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/models/SomeModel.mdl"));

fn create_vbo_and_ibo(graphics_queue: &Arc<Queue>) {

    // Decode file data into instance of wavefront_converter_rs::model::Model
    let model = unsafe {
        Model::from_bytes(&Vec::<u8>::from(SOME_MODEL_BYTES))
    };

    // Create a Vulkano ImmutableBuffer for the vertex buffer (other buffer types should work too)
    let (vbo, _) = {
        let vertex_buffer_usage = BufferUsage {
            vertex_buffer: true,
            ..BufferUsage::none()
        };
        ImmutableBuffer::from_iter(
            model.interleaved_vertices.iter().cloned(),
            vertex_buffer_usage,
            graphics_queue.clone()
        ).unwrap()
    };
    // Note vbo type is  Arc<ImmutableBuffer<[Vertex]>>

    // Create a Vulkano buffer for the index buffer
    let (ibo, _) = {
        let index_buffer_usage = BufferUsage {
            index_buffer: true,
            ..BufferUsage::none()
        };
        ImmutableBuffer::from_iter(
            model.face_indices.iter().cloned(),
            index_buffer_usage,
            graphics_queue.clone()
        ).unwrap()
    };
    // Note ibo type is  Arc<ImmutableBuffer<[u16]>>

    // Do something with vbo and ibo...
}
```
