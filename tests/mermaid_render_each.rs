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
    subgraph Input["Input Registers (Read-Only)"]
        IR0["0: Resonance Freq (Hz×10)"]
        IR1["1: Amplitude (×1000)"]
        IR2["2: Concentration (ppm×10)"]
        IR3["3-4: Timestamp (low/high)"]
        IR5["5: Status Code"]
    end
    
    subgraph Holding["Holding Registers (R/W)"]
        HR0["0: Measurement Interval"]
        HR1["1: Averaging Count"]
        HR2["2: Gain"]
        HR3["3: Filter Strength"]
    end
"#,
    r#"
flowchart TB
    subgraph Public["Public Routes"]
        HOME["/"]
        E404["/*  (404)"]
    end
    
    subgraph Protected["Protected Routes (AuthenticationGuard)"]
        AUDIO["/audio"]
        THERMAL["/thermal"]
        GRAPH["/graph"]
        BLOG["/blog"]
    end
    
    subgraph Auth["Auth Flow"]
        LOGIN["Login"]
        CALLBACK["Callback"]
    end
    
    HOME --> |"Click protected"| LOGIN
    LOGIN --> CALLBACK
    CALLBACK --> Protected
"#,
    r#"
classDiagram
    class AuthProvider {
        <<interface>>
        +isAuthenticated: boolean
        +isLoading: boolean
        +user: AuthUser
        +login(): Promise
        +logout(): Promise
        +getAccessToken(): Promise~string~
        +hasPermission(permission): Promise~boolean~
        +getJson(url): Promise~any~
        +postJson(url, data): Promise~any~
    }
    
    class Auth0Provider {
        -auth0Client: Auth0Client
    }
    
    class GenerixProvider {
        -oidcClient: UserManager
    }
    
    AuthProvider <|-- Auth0Provider
    AuthProvider <|-- GenerixProvider
"#,
    r#"
sequenceDiagram
    participant Component
    participant Hook as useAudioStream
    participant SSE as EventSource
    participant Audio as Web Audio API
    participant Viz as AudioMotion
    
    Component->>Hook: useAudioStream(streamId)
    Hook->>SSE: Connect to /api/audio/stream
    
    loop Every frame (~20ms)
        SSE->>Hook: AudioFrame (JSON/Binary)
        Hook->>Hook: Decode & buffer
        Hook->>Audio: Queue for playback
        Hook->>Viz: Update analyzer
    end
    
    Hook->>Component: { stats, controls, isConnected }
"#,
    r#"
flowchart TB
    subgraph Host["Host Computer"]
        RUST["Rust Backend<br/>(USB-HID Driver)"]
    end
    
    subgraph LaserSmart["Laser+Smart Board"]
        subgraph MCU["ATmega32U4"]
            USB["USB 2.0<br/>HID Device"]
            I2C["I²C Master<br/>400kHz"]
            SPI["SPI Master<br/>4MHz"]
        end
        
        subgraph Analog["Analog Subsystem"]
            ADC1["ADS1115 #1<br/>0x48"]
            ADC2["ADS1115 #2<br/>0x49"]
            ADC3["ADS1115 #3<br/>0x4A"]
            ADC4["ADS1115 #4<br/>0x4B"]
            REF["REF5040<br/>4.096V"]
        end
        
        subgraph Digital["Digital Subsystem"]
            DDS["AD9833<br/>DDS"]
            GPIO["MCP23017<br/>GPIO"]
        end
    end
    
    subgraph External["External Hardware"]
        DTL1["DTL100 #1<br/>Laser + TEC"]
        DTL2["DTL100 #2<br/>Cell TEC"]
        NTC["NTC Sensors<br/>×4"]
        MIC["Microphones<br/>A/B"]
    end
    
    RUST <-->|"USB-HID"| USB
    USB <--> I2C
    USB <--> SPI
    I2C <--> ADC1
    I2C <--> ADC2
    I2C <--> ADC3
    I2C <--> ADC4
    I2C <--> GPIO
    SPI <--> DDS
    SPI <-->|"J5 Connector"| DTL1
    SPI <-->|"J5 Connector"| DTL2
    REF --> ADC1
    REF --> ADC2
    REF --> ADC3
    REF --> ADC4
    NTC --> ADC4
    MIC --> ADC1
"#,
    r#"
sequenceDiagram
    participant Host
    participant MCU as ATmega32U4
    participant ADC as ADS1115
    
    Host->>MCU: READ_ADC [0, 2]
    MCU->>ADC: I2C: Config register (single-shot, A2)
    MCU->>ADC: I2C: Start conversion
    MCU->>MCU: Wait ~1.2ms (860 SPS)
    ADC->>MCU: I2C: Conversion result (16-bit)
    MCU->>Host: HID: [value_h, value_l]
"#,
    r#"
sequenceDiagram
    participant MCU as ATmega32U4
    participant DAC as LTC2641 (DTL100)
    
    Note over MCU,DAC: Set TEC temperature setpoint
    MCU->>DAC: CS_TEC = LOW
    MCU->>DAC: SPI: [0x30, value_h, value_l]
    MCU->>DAC: CS_TEC = HIGH
    Note over DAC: DAC output updated
"#,
    r#"
flowchart LR
    subgraph Semester1["Step 1"]
        A1["Git Basics"]
        A2["Rust Fundamentals"]
        A3["TypeScript/React Intro"]
    end
    
    subgraph Semester2["Step 2"]
        B1["Systems Programming"]
        B2["Network Protocols"]
        B3["Database Integration"]
    end
    
    subgraph Advanced["Advanced Topics"]
        C1["Concurrency"]
        C2["Signal Processing"]
        C3["Hardware Interfaces"]
    end
    
    A1 --> A2 --> B1 --> C1
    A3 --> B2 --> C2
    B3 --> C3
"#,
    r#"
sequenceDiagram
    participant Client
    participant Server
    participant Auth as Auth Service
    
    Note over Client,Auth: Authentication Flow
    Client->>Server: POST /oauth/token {credentials}
    Server->>Auth: Validate credentials
    Auth-->>Server: User valid
    Server-->>Client: JWT Token
    
    Note over Client,Server: API Request
    Client->>Server: GET /api/data<br/>Authorization: Bearer <token>
    Server->>Server: Validate JWT signature
    Server->>Server: Check expiration
    Server->>Server: Extract claims
    Server-->>Client: Protected data
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

#[cfg(feature = "mermaid")]
#[test]
fn render_each_mermaid_block_to_pdf() {
    use std::fs;
    use std::path::PathBuf;

    use genpdfi_extended::{elements, fonts, style, Alignment, Document};

    // Prepare output directory
    let out_dir = PathBuf::from("tests/output/mermaid_render_each");
    fs::create_dir_all(&out_dir).expect("create tests/output/mermaid_render_each dir");

    // Load font
    let font_data = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fonts/NotoSans-Regular.ttf"
    ))
    .to_vec();

    let fd = fonts::FontData::new(font_data, None).expect("font data");
    let family = fonts::FontFamily {
        regular: fd.clone(),
        bold: fd.clone(),
        italic: fd.clone(),
        bold_italic: fd.clone(),
    };

    // Render each Mermaid block to its own PDF
    for (i, mermaid_block) in MERMAID_BLOCKS.iter().enumerate() {
        let mut doc = Document::new(family.clone());
        doc.set_title(format!("Mermaid Render Each - Diagram {}", i + 1));

        doc.push(elements::Paragraph::new(
            ""
        ).styled_string(
            format!("Mermaid Diagram {}", i + 1),
            style::Style::new().with_font_size(16).bold(),
        ));
        doc.push(elements::Paragraph::new(""));

        let mer = elements::Mermaid::new(*mermaid_block).with_alignment(Alignment::Center).with_auto_scale(2.0, 0.9);
            doc.push(mer);

        let output_path = out_dir.join(format!("mermaid_diagram_{}.pdf", i + 1));
        eprintln!("Rendering diagram {} -> {}", i + 1, output_path.display());
        match doc.render_to_file(output_path) {
            Ok(()) => {}
            Err(e) => panic!("Failed to render diagram {}: {:?}", i + 1, e),
        }
    }
    // Check if all files were created
    for i in 1..=MERMAID_BLOCKS.len() {
        let output_path = out_dir.join(format!("mermaid_diagram_{}.pdf", i));
        assert!(output_path.exists(), "Output file {} should exist", output_path.display());

        // Regression check: ensure embedded form XObjects (SVG) are placed fully within page height
        // to avoid visual clipping (top of image outside page). Uses lopdf to inspect page content streams.
        // Failing here indicates a coordinate/offset bug that can reintroduce the earlier visual regression.
        fn assert_images_within_page(path: &std::path::Path) {
            use lopdf::{Document, Object};

            let mut doc = Document::load(path).expect("Could not load generated PDF for validation");
            let pages = doc.get_pages();

            for (_pnum, page_id) in pages {
                // Decode page content into operations (safe to continue if content can't be decoded)
                let content = match doc.get_and_decode_page_content(page_id) {
                    Ok(c) => c,
                    Err(_) => lopdf::content::Content { operations: Vec::new() },
                };

                // Retrieve page height from MediaBox (or CropBox fallback)
                let page_obj = doc.get_object(page_id).expect("page object");
                let page_dict = page_obj.as_dict().expect("page dict");
                let media_box_obj = match page_dict.get(b"MediaBox") {
                    Ok(m) => m,
                    Err(_) => page_dict.get(b"CropBox").expect("page box"),
                };

                let page_h = match media_box_obj {
                    Object::Array(arr) => match &arr[3] {
                        Object::Real(v) => *v as f32,
                        Object::Integer(i) => *i as f32,
                        _ => panic!("unexpected MediaBox entry type"),
                    },
                    _ => panic!("unexpected MediaBox format"),
                };

                // Resources -> XObject dictionary for lookup
                let xobjects_opt = match page_dict.get(b"Resources") {
                    Ok(res) => match res.as_dict() {
                        Ok(res_dict) => match res_dict.get(b"XObject") {
                            Ok(xobj) => match xobj.as_dict() {
                                Ok(dict) => Some(dict.clone()),
                                Err(_) => None,
                            },
                            Err(_) => None,
                        },
                        Err(_) => None,
                    },
                    Err(_) => None,
                };

                // Walk content operations: find 'cm' operations followed by '/Name Do' and validate placement
                for (idx, op) in content.operations.iter().enumerate() {
                    if op.operator == "cm" {
                        // Extract the six numbers a b c d e f for the cm matrix
                        let nums: Vec<f32> = op.operands.iter().filter_map(|o| match o {
                            Object::Real(r) => Some(*r as f32),
                            Object::Integer(i) => Some(*i as f32),
                            _ => None,
                        }).collect();

                        if nums.len() == 6 {
                            let f = nums[5]; // the vertical translation in the cm matrix

                            // Next operation is usually Do (invoke XObject)
                            if idx + 1 < content.operations.len() {
                                let next = &content.operations[idx + 1];
                                if next.operator == "Do" {
                                    if let Some(Object::Name(name)) = next.operands.get(0) {
                                        if let Some(xobjs) = &xobjects_opt {
                                            if let Ok(xref) = xobjs.get(name.as_slice()) {
                                                if let &Object::Reference(rid) = xref {
                                                    let xobj = doc.get_object(rid).expect("xobject");
                                                    if let Object::Dictionary(dict) = xobj {
                                                        if let Ok(bbox_obj) = dict.get(b"BBox") {
                                                            if let Object::Array(bbox) = bbox_obj {
                                                                // BBox is [llx lly urx ury] — height is ury
                                                                let bbox_h = match &bbox[3] {
                                                                    Object::Real(v) => *v as f32,
                                                                    Object::Integer(i) => *i as f32,
                                                                    _ => 0.0_f32,
                                                                };
                                                                let top = f + bbox_h;
                                                                assert!(
                                                                    top <= page_h + 0.01,
                                                                    "Clipped image in {}: placed top={} page_h={} (XObject {:?})",
                                                                    path.display(),
                                                                    top,
                                                                    page_h,
                                                                    String::from_utf8_lossy(name),
                                                                );
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        assert_images_within_page(&output_path);
    }
}