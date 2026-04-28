//! seedance — ByteDance Seedance 2.0 영상 생성 CLI (단일 Rust 바이너리, fal.ai backend).
//! POST queue.fal.run/.../{text-to-video|image-to-video} → polling → mp4 다운로드.
use anyhow::{Context, Result, bail};
use clap::{Parser, ValueEnum};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

const SUBMIT_T2V: &str = "https://queue.fal.run/bytedance/seedance-2.0/text-to-video";
const SUBMIT_I2V: &str = "https://queue.fal.run/bytedance/seedance-2.0/image-to-video";

#[derive(Copy, Clone, Debug, ValueEnum)]
enum Resolution {
    /// 720p (default, 빠름·저비용)
    R720p,
    /// 1080p (시네마틱)
    R1080p,
}

impl Resolution {
    fn as_str(self) -> &'static str {
        match self {
            Resolution::R720p => "720p",
            Resolution::R1080p => "1080p",
        }
    }
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum AspectRatio {
    /// 16:9 가로 (default)
    R16x9,
    /// 9:16 세로 (Threads/Reels)
    R9x16,
    /// 1:1 정사각
    R1x1,
    /// 4:3 클래식
    R4x3,
}

impl AspectRatio {
    fn as_str(self) -> &'static str {
        match self {
            AspectRatio::R16x9 => "16:9",
            AspectRatio::R9x16 => "9:16",
            AspectRatio::R1x1 => "1:1",
            AspectRatio::R4x3 => "4:3",
        }
    }
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum Preset {
    /// 시네마틱 영화 톤 — 1080p 16:9 5s + audio
    Cinematic,
    /// 토킹 헤드 (사람 얼굴 위주) — 1080p 9:16 5s + audio
    Talking,
    /// 제품 광고 (UGC) — 1080p 9:16 5s + audio
    Product,
    /// 애니메이션 — 720p 16:9 5s
    Anime,
}

impl Preset {
    fn resolution(self) -> Resolution {
        match self {
            Preset::Anime => Resolution::R720p,
            _ => Resolution::R1080p,
        }
    }
    fn aspect(self) -> AspectRatio {
        match self {
            Preset::Cinematic | Preset::Anime => AspectRatio::R16x9,
            Preset::Talking | Preset::Product => AspectRatio::R9x16,
        }
    }
    fn audio(self) -> bool { !matches!(self, Preset::Anime) }
}

#[derive(Parser, Debug)]
#[command(version, about = "seedance — ByteDance Seedance 2.0 영상 생성 CLI (fal.ai)")]
struct Args {
    /// Prompt text (한국어 OK)
    prompt: String,
    /// Output path (default: <auto-dir>/seedance-<epoch>.mp4)
    #[arg(long, short = 'o')]
    out: Option<PathBuf>,
    /// Image-to-Video: HTTPS URL 또는 file 경로 (file 은 fal storage 자동 업로드)
    #[arg(long)]
    image: Option<String>,
    /// Preset: cinematic / talking / product / anime
    #[arg(long, value_enum)]
    preset: Option<Preset>,
    /// Duration seconds (5 | 10), preset/CLI override
    #[arg(long, default_value = "5")]
    duration: u32,
    /// Resolution (preset override)
    #[arg(long, value_enum)]
    resolution: Option<Resolution>,
    /// Aspect ratio (preset override)
    #[arg(long, value_enum)]
    aspect: Option<AspectRatio>,
    /// Generate native audio (Seedance 2.0)
    #[arg(long)]
    audio: bool,
    /// Force audio OFF (preset 의 audio 강제 비활성)
    #[arg(long, conflicts_with = "audio")]
    no_audio: bool,
    /// Polling timeout 초 (default 600)
    #[arg(long, default_value = "600")]
    timeout: u64,
    /// Polling 주기 초 (default 5)
    #[arg(long, default_value = "5")]
    poll_interval: u64,
    /// Print only path (scripts)
    #[arg(long)]
    quiet: bool,
}

#[derive(Serialize)]
struct ReqT2V<'a> {
    prompt: &'a str,
    duration: u32,
    resolution: &'a str,
    aspect_ratio: &'a str,
    generate_audio: bool,
}

#[derive(Serialize)]
struct ReqI2V<'a> {
    prompt: &'a str,
    image_url: String,
    duration: u32,
    resolution: &'a str,
    aspect_ratio: &'a str,
    generate_audio: bool,
}

#[derive(Deserialize, Debug)]
struct SubmitResp {
    request_id: String,
    #[serde(default)]
    status_url: Option<String>,
    #[serde(default)]
    response_url: Option<String>,
}

#[derive(Deserialize, Debug)]
struct StatusResp {
    status: String,
}

#[derive(Deserialize, Debug)]
struct ResultResp {
    video: VideoOut,
    #[serde(default)]
    seed: Option<u64>,
}

#[derive(Deserialize, Debug)]
struct VideoOut {
    url: String,
    #[serde(default)]
    duration: Option<f32>,
}

fn fal_key() -> Result<String> {
    if let Ok(k) = std::env::var("FAL_KEY") {
        if !k.is_empty() { return Ok(k); }
    }
    if let Ok(k) = std::env::var("SEEDANCE_API_KEY") {
        if !k.is_empty() { return Ok(k); }
    }
    bail!("FAL_KEY (or SEEDANCE_API_KEY) env var not set")
}

fn default_out_dir(is_termux: bool) -> PathBuf {
    if let Ok(d) = std::env::var("SEEDANCE_OUT_DIR") {
        if !d.is_empty() { return PathBuf::from(d); }
    }
    if is_termux { return PathBuf::from("/sdcard/Movies"); }
    if cfg!(windows) {
        if let Ok(p) = std::env::var("USERPROFILE") {
            return PathBuf::from(p).join("Videos").join("seedance");
        }
    }
    if let Ok(h) = std::env::var("HOME") {
        return PathBuf::from(h).join("Videos").join("seedance");
    }
    PathBuf::from(".")
}

async fn upload_image_to_fal(image: &str, client: &reqwest::Client, key: &str) -> Result<String> {
    if image.starts_with("http://") || image.starts_with("https://") {
        return Ok(image.to_string());
    }
    let path = PathBuf::from(image);
    if !path.exists() { bail!("image not found: {}", image); }
    let bytes = tokio::fs::read(&path).await?;
    let mime = match path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase().as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "webp" => "image/webp",
        _ => "image/png",
    };
    let file_name = path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "img.png".into());
    let init = client.post("https://rest.alpha.fal.ai/storage/upload/initiate")
        .header("Authorization", format!("Key {}", key))
        .json(&serde_json::json!({"content_type": mime, "file_name": file_name}))
        .send().await?;
    if !init.status().is_success() {
        let s = init.status();
        let b = init.text().await.unwrap_or_default();
        bail!("storage initiate {}: {}", s, b);
    }
    #[derive(Deserialize)]
    struct InitResp { upload_url: String, file_url: String }
    let init: InitResp = init.json().await?;
    let put = client.put(&init.upload_url).header("Content-Type", mime).body(bytes).send().await?;
    if !put.status().is_success() { bail!("storage put {}", put.status()); }
    Ok(init.file_url)
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let key = fal_key()?;

    let preset_res = args.preset.map(|p| p.resolution());
    let preset_asp = args.preset.map(|p| p.aspect());
    let preset_audio = args.preset.map(|p| p.audio()).unwrap_or(false);

    let resolution = args.resolution.unwrap_or(preset_res.unwrap_or(Resolution::R720p));
    let aspect = args.aspect.unwrap_or(preset_asp.unwrap_or(AspectRatio::R16x9));
    let generate_audio = if args.no_audio { false } else { args.audio || preset_audio };
    let duration = args.duration;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(180))
        .build()?;

    let (submit_url, body) = if let Some(img) = &args.image {
        let image_url = upload_image_to_fal(img, &client, &key).await?;
        let req = ReqI2V {
            prompt: &args.prompt, image_url,
            duration, resolution: resolution.as_str(),
            aspect_ratio: aspect.as_str(), generate_audio,
        };
        (SUBMIT_I2V, serde_json::to_value(&req)?)
    } else {
        let req = ReqT2V {
            prompt: &args.prompt,
            duration, resolution: resolution.as_str(),
            aspect_ratio: aspect.as_str(), generate_audio,
        };
        (SUBMIT_T2V, serde_json::to_value(&req)?)
    };

    if !args.quiet {
        eprintln!("[seedance] mode={} duration={}s resolution={} aspect={} audio={}",
            if args.image.is_some() { "i2v" } else { "t2v" },
            duration, resolution.as_str(), aspect.as_str(), generate_audio);
    }

    let submit = client.post(submit_url)
        .header("Authorization", format!("Key {}", key))
        .json(&body)
        .send().await
        .context("submit")?;
    let status = submit.status();
    let text = submit.text().await?;
    if !status.is_success() {
        bail!("submit error {}: {}", status, text);
    }
    let s: SubmitResp = serde_json::from_str(&text)
        .with_context(|| format!("parse submit: {}", text.chars().take(300).collect::<String>()))?;

    let status_url = s.status_url.clone()
        .unwrap_or_else(|| format!("{}/requests/{}/status", submit_url, s.request_id));
    let response_url = s.response_url.clone()
        .unwrap_or_else(|| format!("{}/requests/{}", submit_url, s.request_id));

    if !args.quiet {
        eprintln!("[seedance] submitted request_id={}", s.request_id);
    }

    let deadline = std::time::Instant::now() + Duration::from_secs(args.timeout);
    let interval = Duration::from_secs(args.poll_interval);
    loop {
        if std::time::Instant::now() > deadline {
            bail!("timeout after {}s, request_id={}", args.timeout, s.request_id);
        }
        tokio::time::sleep(interval).await;
        let st = client.get(&status_url)
            .header("Authorization", format!("Key {}", key))
            .send().await?;
        let st_text = st.text().await?;
        let st_parsed: StatusResp = match serde_json::from_str(&st_text) {
            Ok(v) => v,
            Err(_) => {
                if !args.quiet { eprintln!("[seedance] status parse skip"); }
                continue;
            }
        };
        if !args.quiet { eprintln!("[seedance] status={}", st_parsed.status); }
        match st_parsed.status.as_str() {
            "COMPLETED" => break,
            "FAILED" | "CANCELLED" => bail!("job {} (request_id={})", st_parsed.status, s.request_id),
            _ => continue,
        }
    }

    let r = client.get(&response_url)
        .header("Authorization", format!("Key {}", key))
        .send().await?;
    let r_text = r.text().await?;
    let result: ResultResp = serde_json::from_str(&r_text)
        .with_context(|| format!("parse result: {}", r_text.chars().take(400).collect::<String>()))?;

    let is_termux = std::env::var("PREFIX").map(|p| p.contains("com.termux")).unwrap_or(false);
    let out: PathBuf = args.out.unwrap_or_else(|| {
        let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        let dir = default_out_dir(is_termux);
        let _ = std::fs::create_dir_all(&dir);
        dir.join(format!("seedance-{}.mp4", ts))
    });

    let resp = client.get(&result.video.url).send().await?;
    if !resp.status().is_success() {
        bail!("video download {}: {}", resp.status(), result.video.url);
    }
    let mut file = tokio::fs::File::create(&out).await?;
    let mut total: u64 = 0;
    let mut stream = resp.bytes_stream();
    use tokio::io::AsyncWriteExt;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        total += chunk.len() as u64;
        file.write_all(&chunk).await?;
    }
    file.flush().await?;

    let path_str = out.to_string_lossy().to_string();
    if is_termux && (path_str.starts_with("/sdcard/") || path_str.starts_with("/storage/")) {
        let _ = std::process::Command::new("su").arg("-c")
            .arg(format!("chmod 644 '{}' && am broadcast -a android.intent.action.MEDIA_SCANNER_SCAN_FILE -d file://{}", path_str, path_str))
            .stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status();
    }

    if args.quiet {
        println!("{}", out.display());
    } else {
        eprintln!("[seedance] saved {} ({} bytes, {}s)", out.display(), total,
            result.video.duration.map(|d| format!("{:.1}", d)).unwrap_or_else(|| "?".into()));
        if let Some(seed) = result.seed { eprintln!("[seedance] seed={}", seed); }
    }
    Ok(())
}
