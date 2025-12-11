# Probar: Missing Features Analysis for Pure Rust WASM Implementation

**Document Version:** 1.0.0
**Status:** Draft Specification
**PMAT Work Reference:** PROBAR-SPEC-001
**Quality Standard:** EXTREME TDD + Toyota Way

---

## Executive Summary

This specification analyzes the top 20 features from Puppeteer and Playwright that are missing from Probar and can be implemented in **pure Rust**. The architecture is divided into two distinct contexts:

1.  **Host CLI** (Running on Linux/macOS/Windows): Orchestrates tests, manages browsers via CDP, and handles file I/O (reporting, media saving).
2.  **WASM Runtime** (Running in Browser): The core test driver injected into the browser, which must be **ABSOLUTE ZERO JavaScript**.

### Constraints

1.  **Pure Rust Only**: No JavaScript, TypeScript, or any JS runtime dependencies.
2.  **WASM-Compatible (Runtime)**: In-browser components must compile to `wasm32-unknown-unknown`.
3.  **CDP-Based (Host)**: Leverage Chrome DevTools Protocol via `chromiumoxide` crate.
4.  **Deterministic**: All operations must support deterministic replay.
5.  **Toyota Way**: Built-in quality (Jidoka), mistake-proofing (Poka-Yoke), continuous improvement (Kaizen).

---

## Toyota Way Quality Principles Applied

| Principle | Application |
|-----------|-------------|
| **Jidoka** (Autonomation) | Fail-fast on quality issues; Andon cord pattern for test failures |
| **Poka-Yoke** (Mistake-Proofing) | Type-safe selectors prevent runtime errors at compile time |
| **Muda** (Waste Elimination) | Zero-copy memory views; lazy evaluation where possible |
| **Heijunka** (Level Loading) | Superblock scheduling for consistent test execution timing |
| **Genchi Genbutsu** (Go and See) | Abstract drivers allow swapping browser/runtime implementations |
| **Kaizen** (Continuous Improvement) | Mutation testing integration; coverage-guided test generation |

---

## Feature Analysis: Top 20 Missing Features

### Category 1: Media Generation & Recording

#### Feature 1: Animated GIF Recording

**Priority:** P0 (Critical)
**Complexity:** Medium
**Pure Rust Feasibility:** ✅ Yes

**Description:** Record test execution as animated GIF for documentation, bug reports, and visual verification.

**Rust Crates Required:**
- `gif` (v0.13) - GIF encoding [1]
- `image` (v0.25) - Image processing

**API Design:**
```rust
pub struct GifRecorder {
    frames: Vec<Frame>,
    config: GifConfig,
}

pub struct GifConfig {
    pub fps: u8,           // 10-30 typical
    pub width: u32,
    pub height: u32,
    pub quality: u8,       // 1-100, palette quantization
    pub loop_count: u16,   // 0 = infinite
}

impl GifRecorder {
    pub fn start(&mut self) -> ProbarResult<()>;
    pub fn capture_frame(&mut self, screenshot: &Screenshot) -> ProbarResult<()>;
    pub fn stop(&mut self) -> ProbarResult<Vec<u8>>;
    pub fn save(&self, path: &Path) -> ProbarResult<()>;
}
```

**Toyota Way Application:**
- **Poka-Yoke**: Type-safe frame capture prevents format mismatches
- **Muda**: Lazy frame encoding reduces memory pressure

**Test Coverage Requirements:** ≥95% line coverage, mutation score ≥80%

---

#### Feature 2: PNG Screenshot Export

**Priority:** P0 (Critical)
**Complexity:** Low
**Pure Rust Feasibility:** ✅ Yes

**Description:** High-quality PNG screenshots with configurable compression and metadata.

**Rust Crates Required:**
- `png` (v0.17) - PNG encoding
- `image` (v0.25) - Image processing

**API Design:**
```rust
pub struct PngExporter {
    compression: CompressionLevel,
    metadata: PngMetadata,
}

pub struct PngMetadata {
    pub title: Option<String>,
    pub description: Option<String>,
    pub timestamp: Option<SystemTime>,
    pub test_name: Option<String>,
}

impl PngExporter {
    pub fn export(&self, screenshot: &Screenshot) -> ProbarResult<Vec<u8>>;
    pub fn export_with_annotations(&self, screenshot: &Screenshot, annotations: &[Annotation]) -> ProbarResult<Vec<u8>>;
    pub fn save(&self, screenshot: &Screenshot, path: &Path) -> ProbarResult<()>;
}
```

---

#### Feature 3: SVG Screenshot Generation

**Priority:** P1 (High)
**Complexity:** Medium
**Pure Rust Feasibility:** ✅ Yes

**Description:** Vector-based screenshots for resolution-independent documentation.

**Rust Crates Required:**
- `svg` (v0.17) - SVG generation
- `resvg` (v0.44) - SVG rendering (optional, for validation)

**API Design:**
```rust
pub struct SvgExporter {
    config: SvgConfig,
}

pub struct SvgConfig {
    pub viewbox: (u32, u32),
    pub preserve_aspect_ratio: bool,
    pub embed_fonts: bool,
    pub compression: SvgCompression,
}

impl SvgExporter {
    pub fn from_screenshot(&self, screenshot: &Screenshot) -> ProbarResult<String>;
    pub fn from_dom_tree(&self, dom: &DomSnapshot) -> ProbarResult<String>;
    pub fn add_annotations(&mut self, annotations: &[Annotation]) -> &mut Self;
}
```

---

#### Feature 4: MP4 Video Recording

**Priority:** P1 (High)
**Complexity:** High
**Pure Rust Feasibility:** ✅ Yes (with constraints)

**Description:** Record test execution as MP4 video for comprehensive test documentation.

**Rust Crates Required:**
- `openh264` (v0.6) - H.264 encoding (pure Rust)
- `mp4` (v0.14) - MP4 container format

**Note:** Full H.264/H.265 encoding in pure Rust has performance implications. Consider:
1. Frame-limited recording (30fps max)
2. Resolution constraints (1080p max)
3. Optional hardware acceleration detection

**API Design:**
```rust
pub struct VideoRecorder {
    encoder: H264Encoder,
    config: VideoConfig,
    frames: Vec<EncodedFrame>,
}

pub struct VideoConfig {
    pub fps: u8,
    pub width: u32,
    pub height: u32,
    pub bitrate: u32,
    pub codec: VideoCodec,
    pub max_duration_secs: u32,
}

impl VideoRecorder {
    pub fn start(&mut self) -> ProbarResult<()>;
    pub fn capture_frame(&mut self, screenshot: &Screenshot) -> ProbarResult<()>;
    pub fn stop(&mut self) -> ProbarResult<Vec<u8>>;
    pub fn save(&self, path: &Path) -> ProbarResult<()>;
}
```

---

### Category 2: CLI & Automation

#### Feature 5: Command-Line Interface

**Priority:** P0 (Critical)
**Complexity:** Medium
**Pure Rust Feasibility:** ✅ Yes

**Description:** Full-featured CLI for running tests, generating reports, and project management.

**Rust Crates Required:**
- `clap` (v4.5) - CLI argument parsing
- `indicatif` (v0.17) - Progress bars
- `console` (v0.15) - Terminal formatting

**CLI Commands:**
```bash
probar test                     # Run all tests
probar test --filter "game::*"  # Filter tests
probar test --parallel 4        # Parallel execution
probar test --coverage          # With coverage
probar test --mutants            # With mutation testing

probar record <test> --gif      # Record as GIF
probar record <test> --mp4      # Record as MP4
probar record <test> --png      # Screenshots only

probar report --html            # Generate HTML report
probar report --junit           # Generate JUnit XML
probar report --lcov            # Generate LCOV coverage

probar init                     # Initialize new project
probar config                   # Manage configuration
```

**API Design:**
```rust
pub struct ProbarCli {
    config: CliConfig,
    runner: TestRunner,
    reporter: Reporter,
}

pub struct CliConfig {
    pub verbosity: Verbosity,
    pub color: ColorChoice,
    pub output_format: OutputFormat,
    pub parallel_jobs: usize,
}

impl ProbarCli {
    pub fn run(args: &[String]) -> ProbarResult<ExitCode>;
    pub fn run_tests(&self, filter: Option<&str>) -> ProbarResult<TestResults>;
    pub fn generate_report(&self, format: ReportFormat) -> ProbarResult<()>;
}
```

---

#### Feature 6: Watch Mode with Hot Reload

**Priority:** P1 (High)
**Complexity:** Medium
**Pure Rust Feasibility:** ✅ Yes

**Description:** Automatic test re-execution on file changes.

**Rust Crates Required:**
- `notify` (v7.0) - File system watching
- `tokio` (v1.42) - Async runtime

**API Design:**
```rust
pub struct WatchMode {
    watcher: FileWatcher,
    debounce_ms: u64,
    patterns: Vec<GlobPattern>,
}

impl WatchMode {
    pub async fn start(&mut self, callback: impl Fn(&[PathBuf])) -> ProbarResult<()>;
    pub fn add_pattern(&mut self, pattern: &str) -> &mut Self;
    pub fn set_debounce(&mut self, ms: u64) -> &mut Self;
}
```

---

### Category 3: Network & Protocol

#### Feature 7: Network Request Interception

**Priority:** P0 (Critical)
**Complexity:** Medium
**Pure Rust Feasibility:** ✅ Yes (via CDP)

**Description:** Intercept and modify network requests/responses for testing.

**Implementation:** Uses CDP's `Fetch` domain.

**API Design:**
```rust
pub struct NetworkInterceptor {
    patterns: Vec<UrlPattern>,
    handlers: HashMap<String, RequestHandler>,
}

pub trait RequestHandler: Send + Sync {
    fn handle(&self, request: &InterceptedRequest) -> InterceptAction;
}

pub enum InterceptAction {
    Continue,
    Modify(ModifiedRequest),
    Fulfill(MockResponse),
    Abort(AbortReason),
}

impl NetworkInterceptor {
    pub fn on_request(&mut self, pattern: &str, handler: impl RequestHandler);
    pub fn mock_response(&mut self, pattern: &str, response: MockResponse);
    pub fn block(&mut self, pattern: &str);
    pub fn throttle(&mut self, pattern: &str, kbps: u32);
}
```

---

#### Feature 8: WebSocket Monitoring

**Priority:** P1 (High)
**Complexity:** Medium
**Pure Rust Feasibility:** ✅ Yes (via CDP)

**Description:** Monitor and mock WebSocket connections.

**API Design:**
```rust
pub struct WebSocketMonitor {
    connections: Vec<WebSocketConnection>,
    handlers: Vec<Box<dyn WebSocketHandler>>,
}

pub trait WebSocketHandler: Send + Sync {
    fn on_message(&self, msg: &WebSocketMessage) -> Option<WebSocketMessage>;
    fn on_close(&self, code: u16, reason: &str);
}

impl WebSocketMonitor {
    pub fn intercept(&mut self, url_pattern: &str, handler: impl WebSocketHandler);
    pub fn send_mock(&mut self, connection_id: u64, message: &[u8]) -> ProbarResult<()>;
    pub fn close(&mut self, connection_id: u64, code: u16) -> ProbarResult<()>;
}
```

---

### Category 4: Tracing & Debugging

#### Feature 9: Execution Tracing

**Priority:** P0 (Critical)
**Complexity:** High
**Pure Rust Feasibility:** ✅ Yes

**Description:** Comprehensive tracing of test execution for debugging.

**Rust Crates Required:**
- `tracing` (v0.1) - Structured logging
- `tracing-subscriber` (v0.3) - Trace collection

**API Design:**
```rust
pub struct ExecutionTracer {
    config: TracingConfig,
    spans: Vec<TracedSpan>,
    events: Vec<TracedEvent>,
}

pub struct TracingConfig {
    pub capture_screenshots: bool,
    pub capture_network: bool,
    pub capture_console: bool,
    pub capture_performance: bool,
}

pub struct TraceArchive {
    pub metadata: TraceMetadata,
    pub spans: Vec<TracedSpan>,
    pub screenshots: Vec<(u64, Screenshot)>,
    pub network_events: Vec<NetworkEvent>,
    pub console_logs: Vec<ConsoleMessage>,
}

impl ExecutionTracer {
    pub fn start(&mut self) -> ProbarResult<()>;
    pub fn stop(&mut self) -> ProbarResult<TraceArchive>;
    pub fn save(&self, path: &Path) -> ProbarResult<()>;
    pub fn export_html(&self) -> ProbarResult<String>;
}
```

---

#### Feature 10: Performance Profiling

**Priority:** P1 (High)
**Complexity:** Medium
**Pure Rust Feasibility:** ✅ Yes (via CDP)

**Description:** Capture performance metrics during test execution.

**API Design:**
```rust
pub struct PerformanceProfiler {
    config: ProfilerConfig,
    metrics: PerformanceMetrics,
}

pub struct PerformanceMetrics {
    pub frame_times: Vec<Duration>,
    pub memory_usage: Vec<MemorySample>,
    pub cpu_usage: Vec<f64>,
    pub network_timing: Vec<NetworkTiming>,
    pub paint_events: Vec<PaintEvent>,
}

impl PerformanceProfiler {
    pub fn start(&mut self) -> ProbarResult<()>;
    pub fn stop(&mut self) -> ProbarResult<PerformanceReport>;
    pub fn get_fps(&self) -> f64;
    pub fn get_frame_budget_violations(&self, target_ms: f64) -> Vec<FrameViolation>;
}
```

---

### Category 5: Code Coverage & Analysis

#### Feature 11: LCOV Report Generation

**Priority:** P0 (Critical)
**Complexity:** Low
**Pure Rust Feasibility:** ✅ Yes

**Description:** Generate LCOV-format coverage reports for CI integration.

**API Design:**
```rust
pub struct LcovReporter {
    coverage_data: CoverageData,
}

impl LcovReporter {
    pub fn from_coverage(data: &CoverageData) -> Self;
    pub fn generate(&self) -> String;
    pub fn save(&self, path: &Path) -> ProbarResult<()>;
}

// LCOV format output:
// TN:<test name>
// SF:<source file>
// FN:<line>,<function name>
// FNDA:<execution count>,<function name>
// FNF:<functions found>
// FNH:<functions hit>
// DA:<line>,<execution count>
// LF:<lines found>
// LH:<lines hit>
// end_of_record
```

---

#### Feature 12: HTML Coverage Report

**Priority:** P0 (Critical)
**Complexity:** Medium
**Pure Rust Feasibility:** ✅ Yes

**Description:** Interactive HTML coverage reports with source code highlighting.

**API Design:**
```rust
pub struct HtmlCoverageReporter {
    coverage_data: CoverageData,
    config: HtmlReportConfig,
}

pub struct HtmlReportConfig {
    pub title: String,
    pub highlight_uncovered: bool,
    pub include_branch_coverage: bool,
    pub theme: Theme,
}

impl HtmlCoverageReporter {
    pub fn generate(&self) -> ProbarResult<String>;
    pub fn save(&self, output_dir: &Path) -> ProbarResult<()>;
}
```

---

#### Feature 13: Cobertura XML Report

**Priority:** P1 (High)
**Complexity:** Low
**Pure Rust Feasibility:** ✅ Yes

**Description:** Cobertura-format coverage reports for CI tools (Jenkins, GitLab CI).

**API Design:**
```rust
pub struct CoberturaReporter {
    coverage_data: CoverageData,
}

impl CoberturaReporter {
    pub fn generate(&self) -> String;
    pub fn save(&self, path: &Path) -> ProbarResult<()>;
}
```

---

### Category 6: Browser Automation Enhancements

#### Feature 14: Multi-Browser Context Management

**Priority:** P1 (High)
**Complexity:** Medium
**Pure Rust Feasibility:** ✅ Yes (via CDP)

**Description:** Manage multiple isolated browser contexts for parallel testing.

**API Design:**
```rust
pub struct BrowserContextManager {
    browser: Browser,
    contexts: HashMap<String, BrowserContext>,
}

pub struct BrowserContext {
    pub id: String,
    pub pages: Vec<Page>,
    pub storage: StorageState,
}

impl BrowserContextManager {
    pub async fn create_context(&mut self, name: &str) -> ProbarResult<&BrowserContext>;
    pub async fn close_context(&mut self, name: &str) -> ProbarResult<()>;
    pub async fn save_storage(&self, context: &str, path: &Path) -> ProbarResult<()>;
    pub async fn load_storage(&mut self, context: &str, path: &Path) -> ProbarResult<()>;
}
```

---

#### Feature 15: Device Emulation

**Priority:** P1 (High)
**Complexity:** Low
**Pure Rust Feasibility:** ✅ Yes (via CDP)

**Description:** Emulate mobile devices, screen sizes, and device capabilities.

**API Design:**
```rust
pub struct DeviceEmulator {
    presets: HashMap<String, DeviceDescriptor>,
}

pub struct DeviceDescriptor {
    pub name: String,
    pub viewport: Viewport,
    pub user_agent: String,
    pub device_scale_factor: f64,
    pub is_mobile: bool,
    pub has_touch: bool,
}

impl DeviceEmulator {
    pub fn iphone_14() -> DeviceDescriptor;
    pub fn ipad_pro() -> DeviceDescriptor;
    pub fn pixel_7() -> DeviceDescriptor;
    pub fn custom(viewport: Viewport, user_agent: &str) -> DeviceDescriptor;
}
```

---

#### Feature 16: Geolocation Mocking

**Priority:** P2 (Medium)
**Complexity:** Low
**Pure Rust Feasibility:** ✅ Yes (via CDP)

**Description:** Mock geolocation for location-based game testing.

**API Design:**
```rust
pub struct GeolocationMock {
    latitude: f64,
    longitude: f64,
    accuracy: f64,
}

impl GeolocationMock {
    pub fn set_position(lat: f64, lon: f64, accuracy: f64) -> Self;
    pub fn new_york() -> Self;
    pub fn tokyo() -> Self;
    pub fn london() -> Self;
    pub async fn apply(&self, page: &Page) -> ProbarResult<()>;
}
```

---

### Category 7: Assertions & Expectations

#### Feature 17: Soft Assertions

**Priority:** P1 (High)
**Complexity:** Low
**Pure Rust Feasibility:** ✅ Yes

**Description:** Collect multiple assertion failures without stopping test execution.

**API Design:**
```rust
pub struct SoftAssertions {
    failures: Vec<AssertionFailure>,
    mode: AssertionMode,
}

impl SoftAssertions {
    pub fn new() -> Self;
    pub fn assert_eq<T: PartialEq + Debug>(&mut self, actual: &T, expected: &T);
    pub fn assert_true(&mut self, condition: bool, message: &str);
    pub fn assert_visible(&mut self, locator: &Locator);
    pub fn verify(&self) -> ProbarResult<()>;
    pub fn failures(&self) -> &[AssertionFailure];
}
```

---

#### Feature 18: Retry Assertions with Polling

**Priority:** P0 (Critical)
**Complexity:** Medium
**Pure Rust Feasibility:** ✅ Yes

**Description:** Auto-retrying assertions for eventually-consistent UI states.

**API Design:**
```rust
pub struct RetryAssertion<T> {
    assertion: Box<dyn Fn() -> AssertionResult>,
    timeout: Duration,
    poll_interval: Duration,
}

impl<T> RetryAssertion<T> {
    pub fn new(assertion: impl Fn() -> AssertionResult + 'static) -> Self;
    pub fn with_timeout(self, timeout: Duration) -> Self;
    pub fn with_poll_interval(self, interval: Duration) -> Self;
    pub async fn verify(&self) -> ProbarResult<()>;
}

// Usage:
// expect!(locator).to_be_visible().with_timeout(5000).verify().await?;
```

---

### Category 8: Advanced Testing Patterns

#### Feature 19: Page Object Model Support

**Priority:** P1 (High)
**Complexity:** Medium
**Pure Rust Feasibility:** ✅ Yes

**Description:** First-class support for Page Object Model pattern.

**API Design:**
```rust
pub trait PageObject {
    fn url_pattern(&self) -> &str;
    async fn navigate(&self, page: &Page) -> ProbarResult<()>;
    async fn is_loaded(&self, page: &Page) -> bool;
}

#[derive(PageObject)]
pub struct LoginPage {
    #[locator("input[name='username']")]
    username_input: Locator,
    #[locator("input[name='password']")]
    password_input: Locator,
    #[locator("button[type='submit']")]
    submit_button: Locator,
}

impl LoginPage {
    pub async fn login(&self, username: &str, password: &str) -> ProbarResult<()> {
        self.username_input.fill(username).await?;
        self.password_input.fill(password).await?;
        self.submit_button.click().await?;
        Ok(())
    }
}
```

---

#### Feature 20: Fixture Management

**Priority:** P1 (High)
**Complexity:** Medium
**Pure Rust Feasibility:** ✅ Yes

**Description:** Test fixture setup and teardown with dependency injection.

**API Design:**
```rust
pub trait Fixture: Send + Sync {
    async fn setup(&mut self) -> ProbarResult<()>;
    async fn teardown(&mut self) -> ProbarResult<()>;
}

pub struct FixtureManager {
    fixtures: HashMap<TypeId, Box<dyn Fixture>>,
    setup_order: Vec<TypeId>,
}

impl FixtureManager {
    pub fn register<F: Fixture + 'static>(&mut self, fixture: F);
    pub async fn setup_all(&mut self) -> ProbarResult<()>;
    pub async fn teardown_all(&mut self) -> ProbarResult<()>;
    pub fn get<F: Fixture + 'static>(&self) -> Option<&F>;
}

// Attribute macro for automatic fixture injection:
#[probar_test]
#[fixture(BrowserFixture)]
#[fixture(DatabaseFixture)]
async fn test_user_login(browser: &Browser, db: &Database) -> ProbarResult<()> {
    // Test implementation
}
```

---

## Implementation Roadmap

### Phase 1: Core Media Generation (Weeks 1-2)
- Feature 1: GIF Recording
- Feature 2: PNG Export
- Feature 5: CLI (basic)

### Phase 2: Coverage & Reporting (Weeks 3-4)
- Feature 11: LCOV Reports
- Feature 12: HTML Coverage
- Feature 13: Cobertura XML

### Phase 3: Advanced Media (Weeks 5-6)
- Feature 3: SVG Export
- Feature 4: MP4 Recording
- Feature 9: Execution Tracing

### Phase 4: Browser Enhancements (Weeks 7-8)
- Feature 7: Network Interception
- Feature 14: Context Management
- Feature 15: Device Emulation

### Phase 5: Testing Patterns (Weeks 9-10)
- Feature 17: Soft Assertions
- Feature 18: Retry Assertions
- Feature 19: Page Object Model
- Feature 20: Fixture Management

### Phase 6: Advanced Features (Weeks 11-12)
- Feature 6: Watch Mode
- Feature 8: WebSocket Monitoring
- Feature 10: Performance Profiling
- Feature 16: Geolocation Mocking

---

## Peer-Reviewed Academic Citations

### Foundational Testing Research

1. Claessen, K., & Hughes, J. (2000). **QuickCheck: A Lightweight Tool for Random Testing of Haskell Programs**. In *Proceedings of the Fifth ACM SIGPLAN International Conference on Functional Programming (ICFP '00)*, pp. 268-279. ACM. https://dl.acm.org/doi/10.1145/357766.351266

2. Goldstein, H., et al. (2024). **Property-Based Testing in Practice**. In *Proceedings of the IEEE/ACM 46th International Conference on Software Engineering*. https://dl.acm.org/doi/10.1145/3597503.3639581

3. Löscher, A., & Sagonas, K. (2017). **Targeted Property-Based Testing**. In *Proceedings of the 26th ACM SIGSOFT International Symposium on Software Testing and Analysis*, pp. 46-56. ACM. https://dl.acm.org/doi/10.1145/3092703.3092711

### Mutation Testing

4. Jia, Y., & Harman, M. (2011). **An Analysis and Survey of the Development of Mutation Testing**. *IEEE Transactions on Software Engineering*, 37(5), 649-678. https://dl.acm.org/doi/10.1109/TSE.2010.62

5. Coles, H., Laurent, T., Henard, C., Papadakis, M., & Ventresque, A. (2016). **PIT: A Practical Mutation Testing Tool for Java**. In *Proceedings of the 25th International Symposium on Software Testing and Analysis (ISSTA)*. https://dl.acm.org/doi/10.1145/2931037.2948707

6. Gopinath, R., et al. (2022). **Mutation Testing in Evolving Systems: Studying the Relevance of Mutants to Code Evolution**. *ACM Transactions on Software Engineering and Methodology*. https://dl.acm.org/doi/10.1145/3530786

### Browser Automation & Web Testing

7. Leotta, M., et al. (2024). **Exploring Browser Automation: A Comparative Study of Selenium, Cypress, Puppeteer, and Playwright**. In *QUATIC 2024*, Springer. https://link.springer.com/chapter/10.1007/978-3-031-70245-7_10

8. Bajammal, M., & Mesbah, A. (2021). **Semantic Web Accessibility Testing via Hierarchical Visual Analysis**. In *2021 IEEE/ACM 43rd International Conference on Software Engineering (ICSE)*, pp. 1610-1621.

9. Wang, W., et al. (2024). **Enhancing the Resiliency of Automated Web Tests with Natural Language**. In *ACM International Conference on AI, Automation and Algorithms*. https://dl.acm.org/doi/10.1145/3700523.3700536

### Visual GUI Testing

10. Alegroth, E., & Feldt, R. (2017). **Visual GUI Testing in Continuous Integration Environment**. *IEEE Conference Publication*. https://ieeexplore.ieee.org/document/7910301/

11. Yusop, N. S. M., et al. (2018). **Continuous Integration and Visual GUI Testing: Benefits and Drawbacks in Industrial Practice**. *IEEE Conference Publication*. https://ieeexplore.ieee.org/document/8367046/

12. Chang, T. H., et al. (2010). **GUI Testing Using Computer Vision**. In *CHI '10: Proceedings of the SIGCHI Conference on Human Factors in Computing Systems*, pp. 1535-1544.

### WebAssembly Testing

13. Haas, A., et al. (2017). **Bringing the Web up to Speed with WebAssembly**. In *Proceedings of the 38th ACM SIGPLAN Conference on Programming Language Design and Implementation*, pp. 185-200. https://dl.acm.org/doi/10.1145/3062341.3062363

14. Hilbig, A., Lehmann, D., & Pradel, M. (2024). **Wasm-R3: Record-Reduce-Replay for Realistic and Standalone WebAssembly Benchmarks**. In *Proceedings of the ACM on Programming Languages (OOPSLA)*. https://dl.acm.org/doi/10.1145/3689787

15. Romano, A., et al. (2024). **Research on WebAssembly Runtimes: A Survey**. *ACM Transactions on Software Engineering and Methodology*. https://dl.acm.org/doi/10.1145/3714465

### Fuzzing & Security Testing

16. Manes, V., et al. (2021). **The Art, Science, and Engineering of Fuzzing: A Survey**. *IEEE Transactions on Software Engineering*, 47(11), 2312-2331.

17. Fioraldi, A., et al. (2020). **AFL++: Combining Incremental Steps of Fuzzing Research**. In *14th USENIX Workshop on Offensive Technologies (WOOT'20)*.

18. Fioraldi, A., et al. (2022). **LibAFL: A Framework to Build Modular and Reusable Fuzzers**. In *ACM SIGSAC Conference on Computer and Communications Security*. https://dl.acm.org/doi/10.1145/3548606.3560602

### Code Coverage

19. Inozemtseva, L., & Holmes, R. (2014). **Coverage is Not Strongly Correlated with Test Suite Effectiveness**. In *Proceedings of the 36th International Conference on Software Engineering*, pp. 435-445.

20. Gopinath, R., Jensen, C., & Groce, A. (2015). **Code Coverage and Test Suite Effectiveness: Empirical Study with Real Bugs in Large Systems**. *IEEE Conference Publication*. https://ieeexplore.ieee.org/document/7081877/

### Accessibility Testing

21. Bajammal, M., & Mesbah, A. (2024). **Enhancing Web Accessibility: Automated Detection of Issues with Generative AI**. *ACM Proceedings on Software Engineering*. https://dl.acm.org/doi/10.1145/3729371

22. Vigo, M., Brown, J., & Conway, V. (2013). **Benchmarking Web Accessibility Evaluation Tools**. In *Proceedings of the 10th International Cross-Disciplinary Conference on Web Accessibility (W4A)*. https://dl.acm.org/doi/10.1145/2461121.2461124

### Toyota Production System in Software

23. Poppendieck, M., & Poppendieck, T. (2003). **Lean Software Development: An Agile Toolkit**. Addison-Wesley Professional.

24. Liker, J. K. (2004). **The Toyota Way: 14 Management Principles from the World's Greatest Manufacturer**. McGraw-Hill.

25. Hibbs, C., Jewett, S., & Sullivan, M. (2009). **The Art of Lean Software Development: A Practical and Incremental Approach**. O'Reilly Media.

### Rust Safety & Formal Methods (Poka-Yoke)

26. Jung, R., Jourdan, J. H., Krebbers, R., & Dreyer, D. (2017). **RustBelt: Securing the Foundations of the Rust Programming Language**. In *Proceedings of the ACM on Programming Languages (POPL)*, 2(POPL), 66. https://dl.acm.org/doi/10.1145/3158154 (The ultimate Poka-Yoke: formal proof of safety).

27. Matsakis, N. D., & Klock, F. S. (2014). **The Rust Language**. In *ACM SIGAda Ada Letters*, 34(3), 103-104. https://dl.acm.org/doi/10.1145/2692956.2663188

28. Balasubramanian, A., Baranowski, M. S., Burtsev, A., Panda, A., Rakamarić, Z., & Ryzhyk, L. (2017). **System Programming in Rust: Beyond Safety**. In *Proceedings of the 16th Workshop on Hot Topics in Operating Systems (HotOS)*, pp. 156-161. https://dl.acm.org/doi/10.1145/3102980.3103006

29. Lamport, L. (2002). **Specifying Systems: The TLA+ Language and Tools for Hardware and Software Engineers**. Addison-Wesley. (Formal specification as mistake-proofing).

### Lean DevOps & Continuous Quality (Jidoka/Kaizen)

30. Forsgren, N., Humble, J., & Kim, G. (2018). **Accelerate: The Science of Lean Software and DevOps: Building and Scaling High Performing Technology Organizations**. IT Revolution Press. (Scientific link between Lean principles and software performance).

31. Humble, J., & Farley, D. (2010). **Continuous Delivery: Reliable Software Releases through Build, Test, and Deployment Automation**. Addison-Wesley. (Foundational text on the "Andon Cord" in software pipelines).

32. Fowler, M. (2006). **Continuous Integration**. *martinfowler.com*. https://martinfowler.com/articles/continuousIntegration.html (Early definition of automated quality gates).

33. Saff, D., & Ernst, M. D. (2004). **An Experimental Evaluation of Continuous Testing During Development**. In *Proceedings of the 2004 International Symposium on Software Testing and Analysis (ISSTA)*, pp. 76-85. https://dl.acm.org/doi/10.1145/1007512.1007523

34. Memon, A., et al. (2017). **Taming Google-Scale Continuous Testing**. In *Proceedings of the 39th International Conference on Software Engineering (ICSE)*, pp. 233-243. https://dl.acm.org/doi/10.1109/ICSE-SEIP.2017.26

35. Bell, J., Legunsen, O., Hilton, M., Eloussi, L., Yung, T., & Marinov, D. (2018). **DeFlaker: Automatically Detecting Flaky Tests**. In *Proceedings of the 40th International Conference on Software Engineering (ICSE)*, pp. 433-444. https://dl.acm.org/doi/10.1145/3180155.3180164 (Crucial for maintaining trust in the Andon cord).

---

## Quality Requirements (EXTREME TDD)

### Test Coverage Requirements

| Module | Line Coverage | Branch Coverage | Mutation Score |
|--------|--------------|-----------------|----------------|
| gif_recorder | ≥95% | ≥90% | ≥80% |
| png_exporter | ≥95% | ≥90% | ≥80% |
| svg_exporter | ≥95% | ≥90% | ≥80% |
| video_recorder | ≥95% | ≥90% | ≥80% |
| cli | ≥95% | ≥90% | ≥80% |
| network_interceptor | ≥95% | ≥90% | ≥80% |
| execution_tracer | ≥95% | ≥90% | ≥80% |
| lcov_reporter | ≥95% | ≥90% | ≥80% |
| All modules | ≥95% | ≥90% | ≥80% |

### Property-Based Testing Requirements

Each feature must include property-based tests using `proptest`:

```rust
proptest! {
    #[test]
    fn prop_gif_frames_preserve_dimensions(width in 1u32..4096, height in 1u32..4096) {
        let config = GifConfig { width, height, ..Default::default() };
        let recorder = GifRecorder::new(config);
        // Property: output dimensions match input
        prop_assert_eq!(recorder.config().width, width);
        prop_assert_eq!(recorder.config().height, height);
    }
}
```

### Mutation Testing Requirements

All features must pass mutation testing with cargo-mutants:

```bash
cargo mutants --package probar --minimum-score 0.80
```

---

## Appendix A: Rust Crate Dependencies

| Feature | Crate | Version | Purpose |
|---------|-------|---------|---------|
| GIF | `gif` | 0.13 | GIF encoding |
| GIF | `image` | 0.25 | Image processing |
| PNG | `png` | 0.17 | PNG encoding |
| SVG | `svg` | 0.17 | SVG generation |
| Video | `openh264` | 0.6 | H.264 encoding |
| Video | `mp4` | 0.14 | MP4 container |
| CLI | `clap` | 4.5 | Argument parsing |
| CLI | `indicatif` | 0.17 | Progress bars |
| CLI | `console` | 0.15 | Terminal formatting |
| Watch | `notify` | 7.0 | File watching |
| Tracing | `tracing` | 0.1 | Structured logging |
| Async | `tokio` | 1.42 | Async runtime |

---

## Appendix B: Non-Goals (Features Requiring JavaScript)

The following features from Puppeteer/Playwright are **explicitly out of scope** as they require JavaScript:

1. **JavaScript Execution** - `page.evaluate()` with JS strings
2. **JavaScript Coverage** - V8's built-in JS coverage
3. **Service Worker Interception** - Requires JS context
4. **Web Workers** - JS-based worker threads
5. **IndexedDB Manipulation** - Browser JS API
6. **LocalStorage Direct Access** - Requires JS context

These features would violate the **ABSOLUTE ZERO JavaScript** constraint.

---

## Document History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0.0 | 2025-01-XX | Probar Team | Initial specification |

---

*This specification follows PMAT Work standards and EXTREME TDD methodology. All implementations must achieve ≥95% test coverage and ≥80% mutation score before merge.*
