use wgpu_test::{gpu_test, image, GpuTestConfiguration, TestParameters, TestingContext};

/// FXC and potentially some glsl consumers have a bug when handling switch statements on a constant
/// with just a default case. (not sure if the constant part is relevant)
/// See <https://github.com/gfx-rs/wgpu/issues/4514>.
///
/// This test will fail on Dx12 with FXC if this issue is not worked around.
///
/// So far no specific buggy glsl consumers have been identified and it isn't known whether the
/// bug is avoided there.
#[gpu_test]
static DEGENERATE_SWITCH: GpuTestConfiguration = GpuTestConfiguration::new()
    .parameters(TestParameters::default().force_fxc(true))
    .run_async(|ctx| async move { test_impl(&ctx).await });

async fn test_impl(ctx: &TestingContext) {
    const TEXTURE_HEIGHT: u32 = 2;
    const TEXTURE_WIDTH: u32 = 2;
    const BUFFER_SIZE: usize = (TEXTURE_WIDTH * TEXTURE_HEIGHT * 4) as usize;

    let texture = ctx.device.create_texture(
        &wgpu::TextureDescriptor::builder()
            .label(Some("Offscreen texture"))
            .size(wgpu::Extent3d {
                width: TEXTURE_WIDTH,
                height: TEXTURE_HEIGHT,
                depth_or_array_layers: 1,
            })
            .format(wgpu::TextureFormat::Rgba8Unorm)
            .usage(wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::RENDER_ATTACHMENT)
            .build(),
    );
    let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    let shader = ctx
        .device
        .create_shader_module(wgpu::include_wgsl!("issue_4514.wgsl"));

    let pipeline = ctx
        .device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Pipeline"),
            layout: None,
            vertex: wgpu::VertexState::from_module(&shader)
                .entry_point("vs_main")
                .build(),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(
                wgpu::FragmentState::from_module(&shader)
                    .entry_point("fs_main")
                    .targets(&[Some(
                        wgpu::ColorTargetState::builder()
                            .format(wgpu::TextureFormat::Rgba8Unorm)
                            .build(),
                    )])
                    .build(),
            ),
            multiview: None,
            cache: None,
        });

    let readback_buffer = image::ReadbackBuffers::new(&ctx.device, &texture);
    {
        let mut encoder = ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Renderpass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        // Important: this isn't the color expected below
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 0.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            render_pass.set_pipeline(&pipeline);
            render_pass.draw(0..3, 0..1);
        }
        readback_buffer.copy_from(&ctx.device, &mut encoder, &texture);
        ctx.queue.submit(Some(encoder.finish()));
    }

    let expected_data = [255; BUFFER_SIZE];
    readback_buffer
        .assert_buffer_contents(ctx, &expected_data)
        .await;
}
