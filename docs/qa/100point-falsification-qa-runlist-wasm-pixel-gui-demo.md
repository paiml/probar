# 100-Point Falsification QA Runlist for PROBAR-SPEC-009

**Target Specification:** PROBAR-SPEC-009 (WASM Pixel GUI Demo)
**Goal:** To rigorously attempt to falsify every claim made in the specification. If a claim cannot be falsified (proven false) through testing, it is considered robust.

## Section 1: Specification & Documentation Integrity (1-10)
1. [ ] **Verify Citation 1 (Nickolls et al., 2008):** Confirm the WGSL compute shader uses the workgroup-based execution model described in the CUDA paper. Falsify by showing the shader relies on non-parallel global synchronization.
2. [ ] **Verify Citation 2 (O'Neill, 2014):** Confirm the RNG implementation matches the PCG-XSH-RR algorithm constants and logic. Falsify by demonstrating statistical failure in Dieharder tests typical of linear congruential generators but passed by PCG.
3. [ ] **Verify Citation 3 (Wilson, 1927):** Check the Wilson Score Interval implementation against the mathematical formula. Falsify by providing edge cases (n=0, p=0, p=1) where the implemented formula diverges from the standard definition.
4. [ ] **Verify Citation 4 (W3C, 2021):** Confirm usage of WebGPU standard APIs (dispatch, bind groups) via `trueno`. Falsify by showing reliance on proprietary or non-standard browser extensions.
5. [ ] **Verify Citation 5 (Mahajan et al., 2021):** Confirm the visual testing approach aligns with "pixel-based" methodology. Falsify by showing the coverage metric ignores pixel-level data in favor of DOM/component-level analysis.
6. [ ] **Completeness Check:** Verify all "Open Questions" in the spec have been resolved and documented in the final implementation plan.
7. [ ] **Acceptance Criteria Mapping:** Confirm each of the 7 Acceptance Criteria has at least one specific test case.
8. [ ] **Version Consistency:** Verify the spec version (v1.0.0) matches the implemented crate version.
9. [ ] **Architecture Diagram Accuracy:** Verify the `trueno` -> `probar` -> `ratatui` data flow matches the actual code dependencies.
10. [ ] **Typographical Audit:** Ensure no misleading typos in variable names or API calls in the spec code snippets (e.g., `gpu-wasm` feature name).

## Section 2: GPU Backend & WebGPU Integration (11-20)
11. [ ] **Workgroup Size Falsification:** Change workgroup size from 256 to 128/512 in `random_fill.wgsl` and verify if performance/correctness degrades or if the spec claim is arbitrary.
12. [ ] **Buffer Binding Validation:** Verify `seed_buffer` is read-only and `pixel_buffer` is read-write in the shader. Falsify by attempting to write to `seed_buffer`.
13. [ ] **1080p Resolution Check:** Assert the pixel buffer is exactly 1920x1080 (2,073,600 elements). Falsify by passing a 1919x1080 buffer and checking for panic or shader error.
14. [ ] **WebGPU Feature Flag:** Verify the code fails to compile or run if the `trueno/gpu-wasm` feature is missing on WASM targets.
15. [ ] **Shader Syntax:** Run `naga` or `wgpu` validation on `random_fill.wgsl` to ensure it is valid WGSL.
16. [ ] **Parallel Execution:** Verify pixels are filled non-sequentially (indicating parallel execution). Falsify by logging write order if possible or checking specific patterns.
17. [ ] **Uniform Updates:** Verify `frame` and `fill_probability` uniforms update every frame. Falsify by freezing them and observing static output.
18. [ ] **Device Loss Handling:** Simulate a GPU device loss (if possible via mock) and verify the application crashes gracefully or recovers, rather than hanging.
19. [ ] **Resource Cleanup:** Verify GPU buffers are destroyed when the demo exits. Falsify by checking for memory leaks in long-running tests.
20. [ ] **Backend Agnostic:** Verify the "native" build uses Vulkan/Metal/DX12 (via `wgpu` defaults) and not a software fallback, unless requested.

## Section 3: Random Number Generation (RNG) (21-30)
21. [ ] **Determinism Check (Same Seed):** Run the demo twice with the same seed. Falsify if *any* pixel differs between the two runs.
22. [ ] **Determinism Check (Different Seeds):** Run with two different seeds. Falsify if the pixel patterns are identical.
23. [ ] **Frame Dependency:** Verify the RNG state depends on the frame number. Falsify by forcing frame=0 loop and observing if the image changes.
24. [ ] **Spatial Independence:** Verify pixel (x,y) does not correlate with (x+1, y). Falsify using autocorrelation tests on the output buffer.
25. [ ] **Uniform Distribution:** Verify that over time, the fill is uniform across the screen. Falsify if "hotspots" or "dead zones" appear persistently.
26. [ ] **Probability Control:** Set `fill_probability` to 0.0. Falsify if any pixels turn on.
27. [ ] **Probability Control:** Set `fill_probability` to 1.0. Falsify if any pixels remain off (in the first frame).
28. [ ] **Seed Buffer Size:** Verify the seed buffer is large enough to avoid visible repetition patterns.
29. [ ] **PCG Constants:** verify the constants `747796405u` etc. are exactly as in the O'Neill paper. Falsify by bit-flipping a constant and checking for RNG quality degradation.
30. [ ] **Zero State:** Verify behavior when seed/input is 0. PCG should handle 0 correctly (unlike some simple LCGs).

## Section 4: TUI Rendering & Visualization (31-40)
31. [ ] **Resolution Downsampling:** Verify the TUI runs on a standard 80x24 terminal. Falsify by resizing terminal to 80x24 and checking for panic.
32. [ ] **Aspect Ratio Preservation:** Verify the heatmap doesn't look stretched. Falsify by visually comparing a circle drawn on GPU vs TUI.
33. [ ] **Color Mapping:** Verify `0.0` maps to background and `>0.0` maps to a color. Falsify by asserting a filled pixel renders as transparent/black.
34. [ ] **Header Rendering:** Verify the header text "WASM Pixel GUI Demo..." is present.
35. [ ] **Stats Update:** Verify the stats text updates in real-time. Falsify by checking if the text remains static while the heatmap changes.
36. [ ] **Unicode Block Usage:** Verify usage of half-blocks or appropriate Unicode characters for higher resolution TUI.
37. [ ] **Flicker Test:** Verify no full-screen clears cause flickering. (Double buffering check).
38. [ ] **Terminal Resize:** Resize the terminal window during execution. Falsify if the application panics or layout breaks permanently.
39. [ ] **Headless Mode:** Verify TUI code can be disabled or mocked for headless CI environments.
40. [ ] **Palette Consistency:** Verify the TUI colors match the `Palette::Viridis` specification (perceptually or by code check).

## Section 5: Statistical Validity (41-50)
41. [ ] **Wilson CI Lower Bound:** Verify `lower` <= `percentage`. Falsify if `lower` > `percentage`.
42. [ ] **Wilson CI Upper Bound:** Verify `upper` >= `percentage`. Falsify if `upper` < `percentage`.
43. [ ] **CI Width:** Verify the CI narrows as `total` (samples) increases. Falsify if CI width is constant or grows.
44. [ ] **Coverage Calculation:** Manually count non-zero pixels in a small buffer and compare with `coverage_stats()`. Falsify if they mismatch.
45. [ ] **Zero Coverage:** Verify stats when buffer is empty (0%).
46. [ ] **Full Coverage:** Verify stats when buffer is full (100%).
47. [ ] **Convergence Rate:** Verify the "Time to 100% coverage < 10s" claim. Falsify by running on reference hardware and timing it.
48. [ ] **Confidence Level:** Verify the formula uses the z-score for 95% confidence (approx 1.96).
49. [ ] **Precision:** Verify stats are reported to at least 2 decimal places as shown in the mockup.
50. [ ] **Independence Assumption:** Critique if Wilson score is appropriate here (it assumes independent Bernoulli trials). Document if pixel spatial correlation violates this and if it matters.

## Section 6: Probar Integration (51-60)
51. [ ] **Gate Creation:** Verify `FalsifiabilityGate::new(15, 25)` is called with correct parameters.
52. [ ] **Threshold Hypothesis:** Verify the 0.95 threshold triggers a pass/fail correctly. Falsify by mocking stats with 0.94 and asserting failure.
53. [ ] **Gap Size Hypothesis:** Verify the max gap size check works. Falsify by creating a buffer with a 101-pixel gap and asserting failure.
54. [ ] **Gate Evaluation:** Verify `gate.evaluate()` returns the correct result based on hypotheses.
55. [ ] **PNG Export Generation:** Verify `export_coverage_snapshot` creates a file.
56. [ ] **PNG Dimensions:** Verify the exported PNG is 1920x1080.
57. [ ] **PNG Content:** Verify the PNG is not black/empty. Falsify by checking file size or reading pixels.
58. [ ] **Legend Export:** Verify the exported PNG includes a legend if `with_legend(true)` is set.
59. [ ] **Snapshot Path:** Verify the export path is respected. Falsify by checking if file appears in CWD instead of specified folder.
60. [ ] **Async Export:** Verify export doesn't block the main render loop for too long (async/await check).

## Section 7: Performance & Benchmarking (61-70)
61. [ ] **60 FPS Target:** Measure loop duration. Falsify if average frame time > 16.7ms on reference GPU.
62. [ ] **Throughput Calculation:** Verify "124M px/s" claim (1920*1080*60). This is a theoretical peak; verify actual throughput is close.
63. [ ] **Memory Profiling (Native):** specific check for < 100MB RAM usage. Falsify if it spikes to GBs.
64. [ ] **Memory Profiling (WASM):** Check WASM heap size. Falsify if it grows unbounded (leak).
65. [ ] **CPU Usage:** Verify CPU usage is low (indicating GPU offload). Falsify if one CPU core is pegged at 100%.
66. [ ] **Startup Time:** Measure time from launch to first frame. Should be reasonable (<1s).
67. [ ] **Buffer Transfer Latency:** Verify "Persistent buffer" approach is used. Falsify by finding `map_read` calls every frame (slow).
68. [ ] **Fill Probability Impact:** Verify lower probability (0.001) doesn't degrade FPS (should be same compute load).
69. [ ] **High Load Test:** Increase probability to 1.0. Verify FPS stays stable.
70. [ ] **Background execution:** Verify behavior when window/tab is backgrounded (browser throttling).

## Section 8: WASM/Browser Compatibility (71-80)
71. [ ] **Wasm-Pack Build:** Verify `wasm-pack build --target web` succeeds.
72. [ ] **Browser Load:** Verify the generated WASM loads in a basic HTML page.
73. [ ] **WebGPU Support:** Verify it detects WebGPU availability. Falsify by running in a browser without WebGPU and ensuring a clear error message (not silent fail).
74. [ ] **Console Logging:** Verify `web-sys` console logging works for debugging.
75. [ ] **Canvas Binding:** Verify the output binds to an HTML Canvas (if visual) or just internal computation. (Spec implies TUI, so maybe xterm.js or just console output? Clarify). *Self-Correction: Spec says "TUI Demo", in browser this likely means xterm.js or pre/code block rendering.*
76. [ ] **No std::thread:** Verify no uses of `std::thread` which panics in WASM.
77. [ ] **Async Runtime:** Verify `wasm-bindgen-futures` is used for async execution.
78. [ ] **File System Access:** Verify no `std::fs` calls in WASM path (snapshots should probably be downloaded blobs).
79. [ ] **Time Source:** Verify usage of `instant` or `web-sys` performance for timing, not `std::time`.
80. [ ] **Asset Loading:** Verify no runtime asset loading that would fail in WASM (embed shaders).

## Section 9: Error Handling & Edge Cases (81-90)
81. [ ] **Invalid Dimensions:** Try 0x0 size. Falsify if panic.
82. [ ] **Huge Dimensions:** Try 8k resolution. Falsify if crash (should error gracefully on VRAM limit).
83. [ ] **Negative Probability:** Try -0.5. Should clamp or error.
84. [ ] **Probability > 1:** Try 1.5. Should clamp or error.
85. [ ] **Missing Palette:** Verify default fallback if palette is somehow invalid.
86. [ ] **Concurrent Access:** If `GpuPixelBuffer` is shared, verify thread safety (Mutex/RwLock).
87. [ ] **Interrupt Handling:** Ctrl+C (SIGINT). Verify clean shutdown (restore terminal).
88. [ ] **Panic Hook:** Verify a panic hook restores the terminal state (cursor, raw mode) so user isn't stuck.
89. [ ] **Empty Seed Buffer:** Verify behavior if seed buffer is empty.
90. [ ] **Shader Compilation Error:** Inject a syntax error in shader at runtime (if loaded dynamically) or compile time. Verify error reporting.

## Section 10: Reproducibility & CI/CD (91-100)
91. [ ] **CI Pipeline:** Verify a CI job exists to run `cargo test --example wasm_pixel_gui_demo`.
92. [ ] **WASM CI:** Verify a CI job runs `wasm-pack test`.
93. [ ] **Linting:** Verify `cargo clippy` passes on the new example code.
94. [ ] **Formatting:** Verify `cargo fmt` checks pass.
95. [ ] **Dependency Check:** Verify `cargo deny` allows the new dependencies (`trueno`, `ratatui`, `wgpu`).
96. [ ] **License Compliance:** Verify all new deps have compatible licenses.
97. [ ] **Golden Image storage:** Verify where golden images are stored (LFS? Git?).
98. [ ] **Platform Parity:** Verify tests pass on both Linux and Windows (if supported).
99. [ ] **Documentation Links:** Verify links in `docs/specifications` point to valid files.
100. [ ] **Falsification Report:** Produce a report summarizing which of these 100 checks passed/failed during development.

---
**Verified By:** ____________________
**Date:** ________________
