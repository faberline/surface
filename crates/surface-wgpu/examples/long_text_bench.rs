//! Long, growing CJK text benchmark (issue #107, part 1: measurement only).
//!
//! Run it with `sh scripts/bench-long-text.sh`. It builds a chat log of about
//! 5,000 Traditional Chinese characters, appends text at 20 appends per second
//! of simulated time, and times each frame through the path a real app uses:
//! build the MicroDom, `LayoutEngine::compute`, `DisplayList::compile` and
//! `Renderer::render`. Frames are not paced. The clock is simulated, so the
//! benchmark measures cost per frame and not a frame rate.
//!
//! It has no threshold and no pass or fail. It needs a real GPU adapter. With
//! no adapter, or a software adapter, it prints "did not run" and exits 2.

use std::time::Instant;
use surface::{Color, DisplayList, LayoutEngine, Length, MicroDom, Node, Role, Style};
use surface_wgpu::Renderer;

const TOTAL_CHARS: usize = 5000;
/// Characters per append. 20 appends per second is 160 characters per second.
const CHARS_PER_APPEND: usize = 8;
const APPENDS_PER_SECOND: usize = 20;
const FRAMES: usize = TOTAL_CHARS / CHARS_PER_APPEND;
/// A chat message is closed after this many characters. The next append opens a new one.
const MESSAGE_CHARS: usize = 250;
const WIDTH: f32 = 800.;
const PADDING: f32 = 20.;
const FONT: f32 = 16.;
const LINE_HEIGHT: f32 = 24.;

/// Deterministic Traditional Chinese text. No file and no download.
fn cjk(len: usize) -> Vec<char> {
    const POOL: &str = "的一是不了人我在有他這中大來上國個到說們為子和你地出道也時年得就那要下以生會自著去之過家學對可她裡後小麼心多天而能好都然沒日於起還發成事只作當想看文無開手十用主行方又如前所本見經頭面公同三已老從動兩長知民樣現分將外但身些與高意進把法此實回二理美點月明其種聲全工己話兒者向情部正名定女問力機給等幾很業最間新什打便位因重被走電四第門相次東政海口使教加少並";
    let pool: Vec<char> = POOL.chars().collect();
    let mut state = 0x2545_f491_u32;
    let mut out = Vec::with_capacity(len);
    while out.len() < len {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        // Every 24th character is a full-width comma or full stop, so lines break like prose.
        out.push(match (state >> 24) % 24 {
            0 => '，',
            1 => '。',
            _ => pool[(state >> 8) as usize % pool.len()],
        });
    }
    out
}

#[derive(Clone, Copy)]
struct Scenario {
    name: &'static str,
    /// Characters per message node. `TOTAL_CHARS` makes one node.
    message_chars: usize,
    /// Viewport height. The tall one shows the whole log.
    height: f32,
    /// False: build the full log first and render the same frame again.
    grow: bool,
}

const SCENARIOS: [Scenario; 6] = [
    Scenario {
        name: "chat, tall viewport, growing",
        message_chars: MESSAGE_CHARS,
        height: 3600.,
        grow: true,
    },
    Scenario {
        name: "chat, tall viewport, static",
        message_chars: MESSAGE_CHARS,
        height: 3600.,
        grow: false,
    },
    Scenario {
        name: "chat, 600 px window, growing",
        message_chars: MESSAGE_CHARS,
        height: 600.,
        grow: true,
    },
    Scenario {
        name: "chat, 600 px window, static",
        message_chars: MESSAGE_CHARS,
        height: 600.,
        grow: false,
    },
    Scenario {
        name: "one node, tall viewport, growing",
        message_chars: TOTAL_CHARS,
        height: 3600.,
        grow: true,
    },
    Scenario {
        name: "one node, tall viewport, static",
        message_chars: TOTAL_CHARS,
        height: 3600.,
        grow: false,
    },
];

fn view(text: &[char], message_chars: usize, height: f32) -> MicroDom {
    let mut root = Style::column()
        .width(Length::Px(WIDTH))
        .height(Length::Px(height))
        .padding(PADDING)
        .gap(8.)
        .background(Color::rgb(254, 247, 255));
    root.clip = true;
    let mut dom = MicroDom::new(Node::new(Role::Group, "log").styled(root));
    let root = dom.root();
    for (i, chunk) in text.chunks(message_chars.max(1)).enumerate() {
        let style = Style::column()
            .width(Length::Px(WIDTH - 2. * PADDING))
            .font_size(FONT)
            .line_height(LINE_HEIGHT);
        let node = Node::new(Role::Text, format!("message {i}"))
            .id(format!("m{i}"))
            .text(chunk.iter().collect::<String>())
            .styled(style);
        dom.append(root, node).unwrap();
    }
    dom
}

#[derive(Default)]
struct Frame {
    build: f64,
    layout: f64,
    compile: f64,
    render_cpu: f64,
    render_gpu: f64,
    ops: usize,
}
impl Frame {
    fn total(&self) -> f64 {
        self.build + self.layout + self.compile + self.render_gpu
    }
}

fn ms(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.
}

fn pct(sorted: &[f64], p: f64) -> f64 {
    let rank = ((sorted.len() as f64 * p).ceil() as usize).clamp(1, sorted.len());
    sorted[rank - 1]
}

fn stats(values: impl Iterator<Item = f64>) -> (f64, f64, f64) {
    let mut v: Vec<f64> = values.collect();
    v.sort_by(f64::total_cmp);
    (pct(&v, 0.5), pct(&v, 0.95), *v.last().unwrap())
}

fn run(
    renderer: &mut Renderer,
    texture: &wgpu::TextureView,
    s: Scenario,
    text: &[char],
) -> Vec<Frame> {
    let mut engine = LayoutEngine::default();
    let mut frames = Vec::with_capacity(FRAMES);
    for frame in 0..FRAMES {
        let shown = if s.grow {
            (frame + 1) * CHARS_PER_APPEND
        } else {
            TOTAL_CHARS
        };
        let start = Instant::now();
        let dom = view(&text[..shown.min(text.len())], s.message_chars, s.height);
        let build = ms(start);
        let start = Instant::now();
        let computed = engine
            .compute(&dom, WIDTH, s.height, &mut renderer.text)
            .expect("layout");
        let layout = ms(start);
        let start = Instant::now();
        let list = DisplayList::compile(&dom, &computed, None);
        let compile = ms(start);
        let start = Instant::now();
        renderer
            .render(&list, texture, WIDTH as u32, s.height as u32, 1.)
            .expect("render");
        let render_cpu = ms(start);
        renderer.device.poll(wgpu::Maintain::Wait);
        let render_gpu = ms(start);
        frames.push(Frame {
            build,
            layout,
            compile,
            render_cpu,
            render_gpu,
            ops: list.ops.len(),
        });
    }
    frames
}

fn report(s: Scenario, frames: &[Frame]) {
    let n = frames.len();
    let tenth = (n / 10).max(1);
    println!("\n## {}", s.name);
    println!(
        "{} frames, {} appends per second of simulated time, {} chars per append, final ops {}",
        n,
        APPENDS_PER_SECOND,
        if s.grow { CHARS_PER_APPEND } else { 0 },
        frames.last().map_or(0, |f| f.ops)
    );
    println!(
        "| phase (ms) | median | p95 | max | first 10% median | last 10% median | last/first |"
    );
    println!("|---|---:|---:|---:|---:|---:|---:|");
    type Pick = fn(&Frame) -> f64;
    let phases: [(&str, Pick); 6] = [
        ("build tree", |f| f.build),
        ("layout (measure)", |f| f.layout),
        ("compile display list", |f| f.compile),
        ("render, CPU return", |f| f.render_cpu),
        ("render, incl. GPU wait", |f| f.render_gpu),
        ("total frame", Frame::total),
    ];
    for (name, pick) in phases {
        let (median, p95, max) = stats(frames.iter().map(pick));
        let (first, _, _) = stats(frames[..tenth].iter().map(pick));
        let (last, _, _) = stats(frames[n - tenth..].iter().map(pick));
        println!(
            "| {name} | {median:.3} | {p95:.3} | {max:.3} | {first:.3} | {last:.3} | {:.1}x |",
            last / first.max(1e-9)
        );
    }
}

fn main() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(&Default::default());
        let adapter = match Renderer::request(&instance, None).await {
            Ok(adapter) => adapter,
            Err(error) => {
                println!("did not run: no GPU adapter ({error})");
                std::process::exit(2);
            }
        };
        let info = adapter.get_info();
        println!("# Long CJK text benchmark");
        println!(
            "adapter: {} ({:?}), backend {:?}",
            info.name, info.device_type, info.backend
        );
        if info.device_type == wgpu::DeviceType::Cpu {
            println!("did not run: the adapter is a software adapter, not a GPU");
            std::process::exit(2);
        }
        let mut renderer = Renderer::new(&adapter, wgpu::TextureFormat::Rgba8Unorm)
            .await
            .expect("renderer");
        let text = cjk(TOTAL_CHARS);
        let missing = renderer
            .text
            .missing_glyphs(&text.iter().collect::<String>());
        println!(
            "text: {} characters, {} missing glyphs, width {} px, font {} px, line height {} px",
            text.len(),
            missing,
            WIDTH,
            FONT,
            LINE_HEIGHT
        );
        assert_eq!(
            missing, 0,
            "the bundled CJK font must cover the benchmark text"
        );
        // Each scenario starts with a fresh LayoutEngine. A discarded pass first warms the
        // font system and the glyph atlas, so no scenario pays the one-time cold start.
        let size = |height: f32| wgpu::Extent3d {
            width: WIDTH as u32,
            height: height as u32,
            depth_or_array_layers: 1,
        };
        for (index, s) in std::iter::once(SCENARIOS[0]).chain(SCENARIOS).enumerate() {
            let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("long text bench"),
                size: size(s.height),
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            let view = texture.create_view(&Default::default());
            let frames = run(&mut renderer, &view, s, &text);
            if index > 0 {
                report(s, &frames);
            }
        }
    });
}
