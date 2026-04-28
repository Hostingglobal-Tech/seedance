# seedance

ByteDance **`Seedance 2.0`** (출시 2026-04-09) 으로 영상을 만드는 작은 Rust CLI. fal.ai backend.
단일 정적 바이너리. Python·Node.js 불필요. 시네마틱 톤 + native audio + 디렉터급 카메라 컨트롤.

[![Rust](https://img.shields.io/badge/Rust-stable-orange)](https://rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue)](LICENSE)

## 이름

**Seedance** — ByteDance 의 Seed 시리즈 (Doubao + Seedance + Seedream) 영상 생성 모델.
Sora · Veo 3 · Kling 2.x 능가 (Artificial Analysis 비디오 아레나 기준).

---

## 설치 — 한 줄

Rust toolchain 설치되어 있으면:

```bash
cargo install --git https://github.com/Hostingglobal-Tech/seedance --locked
```

`FAL_KEY` 환경변수로 fal.ai API key (`<UUID>:<HEX>` 형식) 박아두면 끝.

```bash
echo 'export FAL_KEY="<UUID>:<HEX>"' >> ~/.bashrc
source ~/.bashrc
```

또는 `~/.config/seedance/env` 에 저장 (wrapper 가 자동 로드):

```bash
mkdir -p ~/.config/seedance
echo 'export FAL_KEY="<UUID>:<HEX>"' > ~/.config/seedance/env
chmod 600 ~/.config/seedance/env
```

API key 발급: https://fal.ai/dashboard/keys (계정 + balance 충전 필요)

---

## 사용

```bash
# 기본 (text-to-video, 720p 16:9 5s, audio off)
seedance "벚꽃 흩날리는 한옥 마당, 황혼, 시네마틱"

# 시네마틱 preset (1080p 16:9 + native audio + 5s)
seedance --preset cinematic "강남 야경 드론샷, 자동차 광원 트레일, 영화적 톤"

# UGC 광고 preset (1080p 9:16 + audio) — Threads/Reels 호환
seedance --preset product "삼성 Z Fold 4 손에 들고 화면 전환, 카페, 자연광"

# Image-to-Video
seedance --image /tmp/poster.png --preset talking "카메라 천천히 dolly-in, 표정 미세하게"

# 10초
seedance --duration 10 --preset cinematic "..."

# audio 강제 OFF
seedance --preset cinematic --no-audio "..."

# scripts (path 만 출력)
seedance --quiet -o /tmp/x.mp4 "..."
```

### 옵션

| Flag | 값 | 기본 |
|---|---|---|
| `--preset` | `cinematic` · `talking` · `product` · `anime` | (없음) |
| `--duration` | 5 · 10 | `5` |
| `--resolution` | `r720p` · `r1080p` | preset 또는 `r720p` |
| `--aspect` | `r16x9` · `r9x16` · `r1x1` · `r4x3` | preset 또는 `r16x9` |
| `--audio` | flag | preset 의 audio default |
| `--no-audio` | flag | audio 강제 OFF |
| `--image` | URL 또는 file path | (없음 = t2v) |
| `--timeout` | 초 | `600` |
| `--poll-interval` | 초 | `5` |
| `-o, --out` | 저장 경로 | 아래 표 |
| `--quiet` | — | off |

### Preset

| Preset | Resolution | Aspect | Audio |
|---|---|---|---|
| `cinematic` | 1080p | 16:9 | ON |
| `talking` | 1080p | 9:16 | ON |
| `product` | 1080p | 9:16 | ON |
| `anime` | 720p | 16:9 | OFF |

### 기본 저장 경로

`--out` 생략 시:

| 플랫폼 | 경로 |
|---|---|
| Android (Termux) | `/sdcard/Movies/seedance-<epoch>.mp4` (갤러리 자동 등록) |
| Windows | `%USERPROFILE%\Videos\seedance\` |
| macOS / Linux | `$HOME/Videos/seedance/` |
| 직접 지정 | `export SEEDANCE_OUT_DIR=/원하는/경로` |

### Image-to-Video (i2v)

`--image` 가 HTTPS URL 이면 그대로 전달.
file path 면 fal.ai storage 자동 업로드 후 URL 전달.

```bash
seedance --image https://example.com/poster.jpg --preset talking "표정 미세하게"
seedance --image /tmp/local.png --preset cinematic "카메라 dolly-in"
```

---

## 환경변수

| Var | 의미 |
|---|---|
| `FAL_KEY` | fal.ai API key (`<UUID>:<HEX>`) — 필수 |
| `SEEDANCE_API_KEY` | FAL_KEY 별칭 (호환용) |
| `SEEDANCE_OUT_DIR` | 출력 디렉토리 override |

---

## 가격 (참고, 2026-04 기준)

- Seedance 1.0 Pro: 약 **$0.50 / 5초 1080p** (~3.67 RMB)
- Seedance 2.0: fal.ai 가격표 별도 (https://fal.ai/seedance-2.0)
- 720p 는 1080p 보다 저렴
- balance 부족 시 403 Forbidden — https://fal.ai/dashboard/billing

---

## 모델 — Seedance 2.0 신규

- **native audio** — 영상에 음향·대사 자동 동기화 (preset 자동 ON, `--no-audio` 로 끔)
- **실세계 물리** — 천·물·연기·머리카락 시뮬 자연스러움 ↑
- **디렉터급 카메라 컨트롤** — dolly / pan / orbit / handheld 자연스러운 시네마 톤
- **다국어 prompt** — 한국어 OK
- 모델 ID: `bytedance/seedance/v2/pro/text-to-video` · `image-to-video`

---

## 보안

- API key 코드 하드코딩 X. `FAL_KEY` 환경변수만 사용.
- TLS = `rustls` (OpenSSL 의존 X).
- 텔레메트리·분석·에러 리포팅 X.
- 외부 통신: `queue.fal.run` + `rest.alpha.fal.ai/storage` (i2v 업로드 시) + 결과 mp4 다운로드 호스트.

---

## 라이선스

MIT — [LICENSE](LICENSE)
