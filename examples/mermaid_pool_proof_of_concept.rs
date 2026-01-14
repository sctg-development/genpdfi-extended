//! Proof-of-concept: use a single headless Chrome tab with a TypeScript
//! "mermaid renderer pool" helper page to render multiple Mermaid diagrams
//! concurrently and efficiently.
//!
//! This example demonstrates the Rust side of the handshake protocol that the
//! helper page exposes:
//!
//! - The helper page exposes `window.__mermaidPool.submitTask(id, diagram)`
//!   that returns a Promise resolved with the rendered SVG string.
//! - The helper page also creates a `div#task-<id>` node and sets
//!   `data-state='done'` when the SVG is available. The Rust code waits for
//!   that element and reads its text content.
//!
//! Usage (developer machine):
//!
//! 1. Build the helper bundle:
//!    cd examples/mermaid_pool
//!    npm ci
//!    npm run build
//!
//! 2. Run this example:
//!    cargo run --example mermaid_pool_proof_of_concept --features "mermaid,images"
//!
//! The example will fail with a clear message if `examples/mermaid_pool/dist/index.html`
//! is not found; the build step above produces that file.

// Copyright (c) 2026 Ronan Le Meillat - SCTG Development
// 
// SPDX-License-Identifier: MIT OR Apache-2.0
// Licensed under the MIT License or the Apache License, Version 2.0

#[cfg(not(feature = "mermaid"))]
fn main() {
    eprintln!("This example requires the 'mermaid' feature. Build with --features mermaid");
}

use headless_chrome::{Browser, LaunchOptionsBuilder};
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

/// Number of parallel pools to use in the helper page via ?pool=<N>
const NB_POOLS: usize = 3;

/// A set of mermaid blocks copied from `tests/mermaid_render_each.rs`.
/// For the proof-of-concept we render the same set of diagrams.
const MERMAID_BLOCKS: &[&str] = &[
    r#"
mindmap
  root((Rust-Photoacoustic))
    Signal Processing
      Real-time FFT
      Butterworth filters
      Differential subtraction
      Peak detection
    Hardware Control
      USB-HID interface
      I2C thermal sensors
      SPI DAC/DDS control
      Modbus TCP server
    Web Interface
      Real-time streaming
      OAuth2/JWT security
      Multi-language UI
      Interactive graphs
    Extensibility
      Python integration
      Plugin drivers
      Hot-reload config
      REST API
"#,
    r#"
flowchart TB
    subgraph Hardware["🔧 Hardware Layer"]
        MIC["Microphones A/B"]
        LASER["QCL Laser"]
        TEC["TEC Controllers"]
        NTC["NTC Sensors"]
    end

    subgraph LaserSmart["⚡ Laser+Smart Interface"]
        USB["USB-HID"]
        ADC["16-bit ADC"]
        DAC["12-bit DAC"]
        DDS["DDS Modulator"]
    end

    subgraph Backend["🦀 Rust Backend"]
        ACQ["Acquisition Daemon"]
        PROC["Processing Graph"]
        THERM["Thermal Regulation"]
        API["REST API + SSE"]
        MODBUS["Modbus Server"]
    end

    subgraph Frontend["⚛️ React Frontend"]
        DASH["Dashboard"]
        AUDIO["Audio Analyzer"]
        GRAPH["Processing Graph View"]
        THERMAL["Thermal Monitor"]
    end

    subgraph External["🌐 External Systems"]
        SCADA["SCADA/PLC"]
        REDIS["Redis Pub/Sub"]
        KAFKA["Kafka"]
    end

    MIC --> ADC
    LASER --> DAC
    TEC --> DAC
    NTC --> ADC
    
    ADC --> USB
    DAC --> USB
    DDS --> USB
    USB --> ACQ
    
    ACQ --> PROC
    PROC --> API
    PROC --> MODBUS
    THERM --> API
    
    API --> DASH
    API --> AUDIO
    API --> GRAPH
    API --> THERMAL
    
    MODBUS --> SCADA
    PROC --> REDIS
    PROC --> KAFKA
"#,
    r#"
graph LR
    subgraph Traditional["Traditional Sensors"]
        A[Electrochemical] --> A1["± 1-5% accuracy"]
        B[NDIR] --> B1["ppm resolution"]
        C[Catalytic] --> C1["Cross-sensitivity"]
    end
    
    subgraph LPAS["Laser Photoacoustic"]
        D[QCL Laser] --> D1["ppb resolution"]
        E[Helmholtz Cell] --> E1["Amplified signal"]
        F[Differential] --> F1["Noise rejection"]
    end
    
    style LPAS fill:#90EE90
"#,
    r#"
gantt
    title Revenue Projection (€M)
    dateFormat YYYY
    axisFormat %Y
    
    section R&D Phase
    Development & Patents    :2024, 2025
    
    section Launch Phase
    Lab Product Launch      :2025, 2026
    Industrial Launch       :2026, 2027
    
    section Growth Phase
    Market Expansion        :2027, 2029
"#,
    r#"
pie title Code Distribution
    "Open Source Core" : 70
    "Commercial Plugins" : 20
    "Hardware Designs" : 10
"#,
    r#"
timeline
    title Development Roadmap
    
    Q1 2025 : Hardware prototype v1
            : First customer trials
    
    Q2 2025 : CE certification
            : Production tooling
    
    Q3 2025 : Commercial launch
            : 10 units delivered
    
    Q4 2025 : Series A preparation
            : 30 units backlog
"#,
    r#"
flowchart LR
    subgraph Cell["Differential Helmholtz Cell"]
        subgraph Chamber_A["Chamber A (Excited)"]
            LA["Laser Beam"]
            MA["Microphone A"]
        end
        
        subgraph Neck["Connecting Neck"]
            N["Acoustic Coupling"]
        end
        
        subgraph Chamber_B["Chamber B (Reference)"]
            MB["Microphone B"]
        end
    end
    
    LA --> MA
    MA <--> N
    N <--> MB
    
    style Chamber_A fill:#ffcccc
    style Chamber_B fill:#ccccff
"#,
    r#"
flowchart TB
    subgraph Input["Raw Signals"]
        A["Signal A = PA + Noise"]
        B["Signal B = Noise"]
    end
    
    subgraph Processing["Differential Processing"]
        SUB["A - B"]
    end
    
    subgraph Output["Result"]
        PA["Pure PA Signal"]
    end
    
    A --> SUB
    B --> SUB
    SUB --> PA
    
    style PA fill:#90EE90
"#,
    r#"
flowchart LR
    subgraph Acquisition["1. Acquisition"]
        ADC["48kHz 16-bit ADC"]
    end
    
    subgraph Preprocessing["2. Preprocessing"]
        BP["Bandpass Filter"]
        DIFF["Differential"]
    end
    
    subgraph Spectral["3. Spectral Analysis"]
        WIN["Windowing"]
        FFT["FFT 4096pt"]
        AVG["Averaging"]
    end
    
    subgraph Detection["4. Peak Detection"]
        PEAK["Find f₀"]
        AMP["Extract Amplitude"]
    end
    
    subgraph Output["5. Concentration"]
        CAL["Calibration"]
        CONC["ppm Output"]
    end
    
    ADC --> BP --> DIFF --> WIN --> FFT --> AVG --> PEAK --> AMP --> CAL --> CONC
"#,
    r#"
flowchart TB
    subgraph Input["FFT Magnitude Spectrum"]
        SPEC["mag[0..N/2]"]
    end
    
    subgraph Search["Peak Search"]
        RANGE["Define search range: f₀ ± Δf"]
        MAX["Find local maximum"]
        PARA["Parabolic interpolation"]
    end
    
    subgraph Output["Results"]
        FREQ["Precise frequency"]
        AMP["Amplitude"]
        PHASE["Phase"]
    end
    
    SPEC --> RANGE --> MAX --> PARA --> FREQ
    PARA --> AMP
    PARA --> PHASE
"#,
    r#"
flowchart TB
    subgraph Parameters["Simulation Parameters"]
        F0["Resonance freq: 2000 Hz"]
        Q["Quality factor: 100"]
        T["Temperature: 25°C"]
        C["Concentration: 500 ppm"]
    end
    
    subgraph Model["Physical Model"]
        PA["PA Signal Generation"]
        NOISE["Noise Model"]
        DRIFT["Thermal Drift"]
    end
    
    subgraph Output["Simulated Signals"]
        CH_A["Channel A (Excited)"]
        CH_B["Channel B (Reference)"]
    end
    
    Parameters --> Model --> Output
"#,
    r#"
classDiagram
    class AudioSource {
        <<trait>>
        +name() String
        +sample_rate() u32
        +channels() u16
        +read_frame() Result~AudioFrame~
        +is_realtime() bool
    }
    
    class MicrophoneSource {
        -device: cpal::Device
        -config: StreamConfig
    }
    
    class FileSource {
        -reader: WavReader
        -path: PathBuf
    }
    
    class MockSource {
        -sample_rate: u32
        -frequency: f32
    }
    
    class SimulatedPhotoacousticSource {
        -config: SimulatedSourceConfig
        -resonance_freq: f32
        -concentration: f32
    }
    
    AudioSource <|-- MicrophoneSource
    AudioSource <|-- FileSource
    AudioSource <|-- MockSource
    AudioSource <|-- SimulatedPhotoacousticSource
"#,
    r#"
flowchart TB
    subgraph Graph["ProcessingGraph"]
        INPUT["InputNode"]
        
        subgraph Processing["Processing Nodes"]
            FILTER["FilterNode<br/>(Bandpass)"]
            DIFF["DifferentialNode"]
            GAIN["GainNode"]
        end
        
        subgraph Analytics["Computing Nodes"]
            PEAK["PeakFinderNode"]
            CONC["ConcentrationNode"]
            ACTION["UniversalActionNode"]
        end
        
        subgraph Output["Output Nodes"]
            PA["PhotoacousticOutputNode"]
            STREAM["StreamingNode"]
            RECORD["RecordNode"]
        end
    end
    
    INPUT --> FILTER
    FILTER --> DIFF
    DIFF --> GAIN
    GAIN --> PEAK
    PEAK --> CONC
    CONC --> ACTION
    GAIN --> PA
    GAIN --> STREAM
    GAIN --> RECORD
"#,
    r#"
sequenceDiagram
    participant Source as AudioSource
    participant Daemon as AcquisitionDaemon
    participant Stream as SharedAudioStream
    participant Consumer1 as ProcessingConsumer
    participant Consumer2 as StreamingNode
    
    Source->>Daemon: read_frame()
    Daemon->>Stream: broadcast(frame)
    Stream-->>Consumer1: frame.clone()
    Stream-->>Consumer2: frame.clone()
    Consumer1->>Consumer1: process()
    Consumer2->>Consumer2: encode_sse()
"#,
    r#"
classDiagram
    class ActionDriver {
        <<trait>>
        +execute(measurement: &ActionMeasurement) Result
        +driver_type() String
        +is_available() bool
    }
    
    class RedisActionDriver {
        -client: redis::Client
        -mode: RedisMode
    }
    
    class HttpsCallbackDriver {
        -client: reqwest::Client
        -url: String
    }
    
    class KafkaActionDriver {
        -producer: FutureProducer
        -topic: String
    }
    
    class PythonActionDriver {
        -py_function: PyObject
    }
    
    ActionDriver <|-- RedisActionDriver
    ActionDriver <|-- HttpsCallbackDriver
    ActionDriver <|-- KafkaActionDriver
    ActionDriver <|-- PythonActionDriver
"#,
    r#"
flowchart LR
    subgraph Input
        SP["Setpoint"]
        PV["Process Value<br/>(NTC reading)"]
    end
    
    subgraph PID["PID Controller"]
        E["Error = SP - PV"]
        P["P: Kp × e"]
        I["I: Ki × ∫e dt"]
        D["D: Kd × de/dt"]
        SUM["Σ"]
    end
    
    subgraph Output
        DAC["DAC Output"]
        TEC["TEC Driver"]
    end
    
    SP --> E
    PV --> E
    E --> P --> SUM
    E --> I --> SUM
    E --> D --> SUM
    SUM --> DAC --> TEC
"#,
    r#"
flowchart TB
    subgraph Rocket["Rocket Web Server"]
        subgraph Auth["Authentication"]
            OAUTH["OAuth2 Endpoints"]
            JWT["JWT Validation"]
        end
        
        subgraph API["REST API"]
            CONFIG["GET /api/config"]
            THERMAL["GET /api/thermal"]
            GRAPH["GET /api/graph"]
            COMPUTING["GET /api/computing"]
        end
        
        subgraph SSE["Server-Sent Events"]
            AUDIO["GET /api/audio/stream"]
            SPECTRAL["GET /api/spectral/stream"]
        end
        
        subgraph Static["Static Files"]
            SPA["React SPA"]
            ASSETS["Assets"]
        end
    end
    
    Auth --> API
    Auth --> SSE
"#,
    r#"
sequenceDiagram
    participant Client
    participant Rocket as Rocket Server
    participant OAuth as OAuth2 Provider
    participant JWT as JWT Validator
    
    Client->>Rocket: GET /oauth/authorize
    Rocket->>OAuth: Redirect to login
    OAuth->>Client: Authorization code
    Client->>Rocket: POST /oauth/token
    Rocket->>JWT: Generate JWT
    JWT->>Client: Access token
    Client->>Rocket: GET /api/data (Bearer token)
    Rocket->>JWT: Validate token
    JWT->>Rocket: Claims
    Rocket->>Client: Protected resource
"#,
    r#"
flowchart LR
    subgraph Input
        SIG["Mixed Signal<br/>100Hz + 2000Hz + 5000Hz"]
    end
    
    subgraph Filter["Bandpass Filter<br/>1800-2200 Hz"]
        BP["Butterworth<br/>4th order"]
    end
    
    subgraph Output
        OUT["Filtered Signal<br/>2000Hz only"]
    end
    
    SIG --> BP --> OUT
"#,
];

fn load_helper_page() -> Result<PathBuf, String> {
    // Try multiple likely locations: relative path (when running from repo root)
    // and an absolute path based on CARGO_MANIFEST_DIR (safer when CWD differs).
    let candidates = [
        PathBuf::from("examples/mermaid_pool/dist/index.html"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/mermaid_pool/dist/index.html"),
        PathBuf::from("./dist/index.html"),
    ];

    for p in &candidates {
        if p.exists() {
            return Ok(p.to_path_buf());
        }
    }

    Err(format!(
        "Built helper page not found. Tried: {}. Build via: cd examples/mermaid_pool && npm ci && npm run build",
        candidates
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    ))
}

#[cfg(feature = "mermaid")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Locate the built helper page path
    let helper_path = match load_helper_page() {
        Ok(p) => p,
        Err(msg) => {
            eprintln!("{}", msg);
            std::process::exit(2);
        }
    };

    // Resolve absolute path and use a file:// URL so query params like ?pool=3 work
    let abs = helper_path
        .canonicalize()
        .map_err(|e| format!("Failed to canonicalize {}: {}", helper_path.display(), e))?;

    // Change headless=false for viewing the Chromium window for debugging
    let browser = Browser::new(
        LaunchOptionsBuilder::default()
            .headless(true)
            .build()
            .expect("Failed to build launch options"),
    )
    .expect("Failed to start Chrome");
    let tab = browser.new_tab().expect("Failed to open a new tab");

    // Navigate to the helper page via file:// so query params are parsed correctly
    tab.navigate_to(&format!("file://{}?pool={}", abs.display(), NB_POOLS))?;
    tab.wait_until_navigated()?;

    eprintln!("Helper page loaded; rendering {} diagrams (pool={})...", MERMAID_BLOCKS.len(), NB_POOLS);

    // Wait for the helper page to initialize the pool and publish metrics
    match tab.wait_for_element("#mermaid-metrics") {
        Ok(metrics_node) => {
            let metrics = metrics_node.get_inner_text()?;
            eprintln!("Helper metrics: {}", metrics);
        }
        Err(e) => eprintln!("Warning: mermaid metrics node not found: {}", e),
    }

    let start = Instant::now();
    for (i, diagram) in MERMAID_BLOCKS.iter().enumerate() {
        let id = format!("poc-{}-{}", i + 1, start.elapsed().as_millis());

        // Encode the diagram as a JSON string (properly escaped/unicode-safe)
        let js_diagram = serde_json::to_string(diagram).map_err(|e| format!("Failed to JSON-encode diagram: {}", e))?;
        let submit = format!("window.__mermaidPool.submitTask('{}', {})", id, js_diagram);

        // Evaluate the submit which enqueues the task on the page side
        match tab.evaluate(&submit, true) {
            Ok(_) => {
                eprintln!("Submitted task {}", id);
            }
            Err(e) => {
                eprintln!("Failed to submit task {}: {}", id, e);
                continue;
            }
        }

        // Small delay to avoid flooding the pool too quickly
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Wait for the DOM node `#task-<id>[data-state='done']` to appear and then read its text
        let selector = format!("#task-{}[data-state='done']", id);
        let node = match tab.wait_for_element(&selector) {
            Ok(n) => n,
            Err(e) => {
                eprintln!("Task {} timed out waiting for result: {}", id, e);
                continue;
            }
        };

        let svg = node.get_inner_text()?;
        eprintln!("Diagram {} produced {} bytes", i + 1, svg.len());
        // For POC, save the svg content to a file per diagram. Make the path
        // relative to the crate root so it does not depend on the current CWD.
        let out_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/output/mermaid_pool_poc");
        fs::create_dir_all(&out_dir)?;
        let out_path = out_dir.join(format!("mermaid_poc_{}.svg", i + 1));
        fs::write(&out_path, svg)?;
    }

    eprintln!("Total time: {:.3?}", start.elapsed());
    Ok(())
}
