# UAV Ground Control Station

Phần mềm điều khiển mặt đất (GCS) tùy chỉnh viết bằng Rust cho UAV cánh bằng (fixed-wing).

## Cấu Trúc Dự Án

```
uav/
├── Cargo.toml              # Workspace root
├── crates/
│   ├── uav-protocol/       # MAVLink message types, telemetry data structures
│   ├── uav-core/           # Communication, telemetry processing, gamepad input
│   ├── uav-video/          # Video streaming receiver/decoder (FPV)
│   └── uav-gcs/            # GUI application (egui/eframe)
└── docs/                   # Hardware documentation & guides
```

## Tính Năng

- **MAVLink Communication**: Kết nối ArduPilot qua Serial/UDP/TCP
- **Real-time Telemetry**: Attitude, GPS, battery, airspeed, RC signal
- **Attitude Indicator**: Artificial horizon vẽ bằng egui
- **Flight Mode Control**: Chuyển đổi MANUAL/STAB/FBWA/AUTO/RTL
- **Gamepad Support**: Điều khiển qua gamepad/joystick (RC override)
- **Video Feed**: FPV video receiver (đang phát triển)

## Build & Run

```bash
# Build all crates
cargo build

# Run GCS
cargo run -p uav-gcs

# Run with SITL (ArduPilot simulator)
# 1. Start SITL: sim_vehicle.py -v ArduPlane --map --console
# 2. Run GCS connecting to SITL UDP port 14550
cargo run -p uav-gcs

# Run tests
cargo test
```

## Yêu Cầu Hệ Thống

- Rust 1.75+
- Linux: `sudo apt install libxkbcommon-dev libwayland-dev libudev-dev`
- Build deps cho egui: xorg-dev hoặc tương đương

## Kết Nối với SITL (Mô Phỏng)

```bash
# Cài đặt ArduPilot SITL
git clone https://github.com/ArduPilot/ardupilot.git
cd ardupilot
git submodule update --init --recursive
Tools/environment_install/install-prereqs-ubuntu.sh -y

# Chạy SITL cho ArduPlane
cd ArduPlane
sim_vehicle.py -v ArduPlane --map --console
# SITL output UDP 14550 — GCS tự động kết nối
```

## License

MIT
