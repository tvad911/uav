# Samsung S20 → UAV Companion Computer Module

> Kế hoạch triển khai tái sử dụng Samsung Galaxy S20 hỏng làm **Companion Computer** cho UAV cánh bằng.  
> Tích hợp: 4G data link, AI nhận diện vật thể, quét 3D địa hình, điều khiển bay thông minh.

---

## Đánh Giá Tính Khả Thi

**Hoàn toàn khả thi.** Samsung S20 mạnh hơn Raspberry Pi 4 / Jetson Nano ở nhiều khía cạnh, và đã có sẵn 4G/GPS/Camera/NPU.

### Specs Samsung S20 phục vụ UAV

| Thành phần | Thông số | Vai trò UAV |
|---|---|---|
| SoC | Exynos 990 (8 cores, 2.73GHz) | Xử lý AI, MAVLink routing |
| NPU | Dual-core NPU 15 TOPS | Chạy YOLO/TFLite realtime |
| GPU | Mali-G77 MP11 | OpenCV, render 3D |
| RAM | 8-12GB LPDDR5 | Multiprocess: AI + video + MAVLink |
| Camera | 12MP wide + 64MP tele + 12MP ultra | Quét cảnh vật, photogrammetry |
| 4G/5G | LTE Cat.20 (2Gbps) | Data link GCS ↔ UAV |
| GPS | Dual-band L1+L5 | Geotag ảnh, waypoint |
| USB-C | USB 3.2 Gen 1 | Kết nối FC qua Serial |
| Sensors | Accel, Gyro, Baro, Magnetometer | Hỗ trợ EKF backup |

> **Lưu ý:** Cần xác nhận phần nào của S20 bị hỏng. Nếu chỉ hỏng màn hình/pin → dùng headless qua ADB, cấp nguồn từ UAV battery. Nếu hỏng mainboard/modem → cần thay thế hoặc chuyển sang Raspberry Pi 5.

---

## Kiến Trúc Hệ Thống

```
┌─────────────────────────────────────────────────┐
│                   UAV Onboard                   │
│                                                 │
│  ┌──────────────┐     USB Serial    ┌────────┐  │
│  │ Pixhawk FC   │◄────MAVLink──────►│Samsung │  │
│  │ (ArduPilot)  │                   │  S20   │  │
│  └──────────────┘                   │        │  │
│                                     │Camera2 │  │
│  ┌──────────────┐    UVC / API      │  API   │  │
│  │ USB Camera   │─────────────────► │        │  │
│  │ (optional)   │                   └───┬────┘  │
│  └──────────────┘                       │       │
└─────────────────────────────────────────┼───────┘
                                          │ 4G LTE / VPN
                                          │
┌─────────────────────────────────────────┼───────┐
│               GCS (Ground)              │       │
│                                         ▼       │
│  ┌──────────────────────────────────────────┐   │
│  │ UAV GCS App (Rust/egui)                  │   │
│  │  - MAVLink telemetry                     │   │
│  │  - Video receiver                        │   │
│  │  - AI detection overlay                  │   │
│  │  - Companion status panel                │   │
│  └──────────────────────────────────────────┘   │
└─────────────────────────────────────────────────┘
```

### Luồng dữ liệu

| Luồng | Protocol |
|---|---|
| FC → S20 → GCS (Telemetry) | MAVLink over TCP tunnel |
| GCS → S20 → FC (Commands) | MAVLink over TCP tunnel |
| S20 Camera → GCS (Video) | RTSP/H.264 over 4G |
| S20 AI → GCS (Detection) | JSON over WebSocket |
| S20 Camera → Storage (Photos) | JPEG + EXIF GPS |

---

## Danh Sách Phần Cứng

### Trên UAV (Onboard)

| # | Phần cứng | Mục đích | Giá |
|---|---|---|---|
| 1 | **Samsung S20** (hỏng) | Companion Computer | Đã có |
| 2 | **Pixhawk 2.4.8** / **Matek H743** | Flight Controller | $40-80 |
| 3 | **USB-C OTG Cable** | S20 → FC | $3 |
| 4 | **USB-to-UART TTL** (CP2102/FTDI) | USB → Serial cho FC | $3 |
| 5 | **BEC 5V 3A** (UBEC) | Nguồn S20 từ LiPo | $5 |
| 6 | **USB-C Power Cable** | BEC → S20 | $2 |
| 7 | **SIM 4G** (Viettel/Mobi) | Data link | ~$3/tháng |
| 8 | **3D Printed Mount** | Gắn S20 lên khung | $5 |
| 9 | *(Tùy chọn)* **USB Camera** (ELP/Arducam) | Camera ngoài | $20-60 |
| 10 | *(Tùy chọn)* **USB Hub OTG** | Multi-device | $5 |

### Ground Control Station

| # | Phần cứng | Mục đích |
|---|---|---|
| 1 | PC/Laptop Linux | UAV GCS App |
| 2 | Internet 4G/WiFi | Kết nối VPN tới S20 |
| 3 | *(Tùy chọn)* Gamepad | Điều khiển manual |

### Sơ đồ kết nối vật lý

```
LiPo Battery ──→ BEC 5V 3A ──→ USB-C ──→ Samsung S20
                                              │
                                              ├──→ USB-C OTG → USB-UART → Pixhawk TELEM2
                                              └──→ Camera (built-in hoặc USB via Hub)
```

---

## Phần Mềm Trên Mỗi Thiết Bị

### A. Samsung S20

| # | Phần mềm | Vai trò | Ngôn ngữ |
|---|---|---|---|
| 1 | **LineageOS 20+** (hoặc root stock) | OS headless | — |
| 2 | **Termux** | Terminal Linux | — |
| 3 | **MAVLink Router** | FC ↔ S20 ↔ GCS proxy | C++ |
| 4 | **WireGuard VPN** | Tunnel bảo mật 4G | — |
| 5 | **uav-companion** (Custom) | App chính | Kotlin + Rust JNI |
| 6 | **TFLite** + NPU Delegate | AI inference | Java/C++ |
| 7 | **GStreamer Android** | Video RTSP | C |
| 8 | **OpenCV Android** | Image processing | C++ JNI |

#### Cấu trúc App `uav-companion`

```
uav-companion/
├── app/src/main/java/com/uav/companion/
│   ├── MainActivity.kt            # Headless service
│   ├── mavlink/
│   │   ├── MavLinkService.kt      # USB Serial ↔ MAVLink
│   │   └── TelemetryBridge.kt     # Forward qua 4G
│   ├── camera/
│   │   ├── CameraService.kt       # Camera2 API
│   │   └── VideoStreamer.kt        # RTSP server
│   ├── ai/
│   │   ├── ObjectDetector.kt      # TFLite YOLO
│   │   ├── TerrainMapper.kt       # Photo + geotag
│   │   └── DetectionResult.kt
│   └── network/
│       ├── VpnManager.kt          # WireGuard
│       └── WebSocketServer.kt     # AI results → GCS
├── assets/models/
│   ├── yolov8n.tflite
│   └── depth_estimation.tflite
├── rust-bridge/                   # Rust → .so via JNI
│   ├── Cargo.toml
│   └── src/lib.rs
└── build.gradle
```

### B. Flight Controller (ArduPilot)

```
SERIAL2_PROTOCOL = 2        # MAVLink2
SERIAL2_BAUD = 921          # 921600 baud
BRD_RTC_TYPES = 2           # GPS time sync
LOG_BACKEND_TYPE = 3        # Log flash + companion
SR2_POSITION = 10           # 10Hz position
SR2_EXTRA1 = 10             # 10Hz attitude
SR2_EXTRA3 = 3              # 3Hz sensors
```

### C. GCS App (Rust — đã code sẵn)

Các module đã implement trong `uav-gcs`:
- ✅ `companion_panel.rs` — Monitor S20 (CPU, temp, 4G, AI status)
- ✅ `detection_overlay.rs` — AI bounding boxes trên video
- ✅ `companion.rs` — Protocol types (CompanionStatus, DetectionResult...)

Cần thêm khi triển khai:
- `link_manager.rs` — Quản lý VPN tunnel
- `terrain_viewer.rs` — Xem 3D point cloud/mesh

---

## Tính Năng AI

### Object Detection
- **Model:** YOLOv8n / YOLOv11n (INT8 quantized)
- **Runtime:** TFLite + Samsung NPU delegate (~30 FPS)
- **Classes:** Person, Car, Truck, Bicycle, Building, Animal
- **Output:** Bounding box + label + confidence → JSON → GCS

### 3D Terrain Mapping (Photogrammetry)
- Auto-trigger ảnh theo interval / overlap %
- Geotag: GPS L1+L5 + FC barometer altitude
- Post-process: OpenDroneMap → Orthophoto, DSM, Point Cloud, 3D Mesh

### Flight Intelligence

| Tính năng | Mô tả | Độ khó |
|---|---|---|
| Obstacle Avoidance | Depth estimation → OBSTACLE_DISTANCE msg | ⭐⭐⭐ |
| Visual Landing | ArUco marker → LANDING_TARGET msg | ⭐⭐ |
| Follow Me | Track target → GUIDED waypoint | ⭐⭐ |
| Geofence Alert | AI detect restricted areas → auto RTL | ⭐⭐ |
| Auto Survey | Grid waypoints + auto capture | ⭐⭐ |

---

## Giai Đoạn Triển Khai

### Phase 1: MAVLink Bridge qua 4G *(2-3 tuần)*
- [ ] Flash LineageOS / root S20
- [ ] Setup Termux + MAVLink Router
- [ ] Kết nối S20 → FC qua USB Serial
- [ ] Setup WireGuard VPN tunnel
- [ ] GCS kết nối tới S20 qua 4G
- [ ] Test end-to-end: GCS → 4G → S20 → FC

### Phase 2: Video Streaming *(1-2 tuần)*
- [ ] GStreamer trên S20 (Camera → H.264 → RTSP)
- [ ] GCS nhận video qua 4G (đã có VideoReceiver)
- [ ] Adaptive bitrate theo 4G signal

### Phase 3: Object Detection *(2-3 tuần)*
- [ ] TFLite + YOLOv8n trên S20
- [ ] Camera → detection → JSON WebSocket → GCS
- [ ] GCS hiển thị bounding boxes overlay
- [ ] Fine-tune model cho use case cụ thể

### Phase 4: 3D Mapping *(2-3 tuần)*
- [ ] Auto photo capture + geotag GPS
- [ ] Mission planner: grid survey pattern
- [ ] Download ảnh qua 4G
- [ ] OpenDroneMap 3D reconstruction
- [ ] GCS viewer: point cloud / orthophoto

### Phase 5: Flight Intelligence *(3-4 tuần)*
- [ ] Monocular depth estimation
- [ ] OBSTACLE_DISTANCE MAVLink message
- [ ] Visual landing (ArUco)
- [ ] Follow-me mode
- [ ] Auto-survey mission

---

## Checklist Trước Khi Bắt Đầu

- [ ] Xác nhận S20 hỏng phần nào (màn hình? pin? mainboard?)
- [ ] Xác nhận phiên bản (Exynos / Snapdragon)
- [ ] Có Flight Controller chưa (Pixhawk / Matek?)
- [ ] Có khung UAV chưa
- [ ] Mua phụ kiện: OTG cable, USB-UART, BEC, SIM 4G
- [ ] Cài Android Studio cho development
