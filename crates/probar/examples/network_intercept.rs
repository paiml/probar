//! Example: Network Interception (Feature 7)
//!
//! Demonstrates: Intercepting and mocking HTTP requests
//!
//! Run with: `cargo run --example network_intercept`
//!
//! Toyota Way: Poka-Yoke (Mistake-Proofing) - Type-safe request handling

use probar::prelude::*;

fn main() -> ProbarResult<()> {
    println!("=== Network Interception Example ===\n");

    // 1. Create network interception
    println!("1. Creating network interceptor...");
    let mut interceptor = NetworkInterceptionBuilder::new().build();

    println!("   Interceptor created");

    // 2. Add URL patterns
    println!("\n2. Creating URL patterns...");
    let exact_pattern = UrlPattern::Exact("https://api.example.com/users".into());
    let glob_pattern = UrlPattern::Glob("https://api.example.com/*".into());
    let regex_pattern = UrlPattern::Regex(r"https://.*\.example\.com/.*".into());

    println!("   Exact: {:?}", exact_pattern);
    println!("   Glob: {:?}", glob_pattern);
    println!("   Regex: {:?}", regex_pattern);

    // 3. Create mock response
    println!("\n3. Creating mock response...");
    let mock_response = MockResponse::new()
        .with_status(200)
        .with_header("Content-Type", "application/json")
        .with_body(r#"{"id": 1, "name": "Test User"}"#.as_bytes().to_vec());

    println!("   Status: {}", mock_response.status);
    println!("   Headers: {:?}", mock_response.headers);
    println!("   Body length: {} bytes", mock_response.body.len());

    // 4. Add route with mock
    println!("\n4. Adding route with mock response...");
    let route = Route::new(exact_pattern.clone(), HttpMethod::Get, mock_response);

    interceptor.route(route);
    println!("   Route added");
    println!("   Route count: {}", interceptor.route_count());

    // 5. Simulate captured requests
    println!("\n5. Simulating captured requests...");
    let mut request = CapturedRequest::new("https://api.example.com/users", HttpMethod::Get, 0);
    let _ = request
        .headers
        .insert("Accept".to_string(), "application/json".to_string());

    println!("   URL: {}", request.url);
    println!("   Method: {:?}", request.method);
    println!("   Headers: {:?}", request.headers);

    // 6. HTTP methods
    println!("\n6. HTTP methods...");
    let methods = [
        HttpMethod::Get,
        HttpMethod::Post,
        HttpMethod::Put,
        HttpMethod::Delete,
        HttpMethod::Patch,
        HttpMethod::Head,
        HttpMethod::Options,
    ];

    for method in &methods {
        println!("   {:?}", method);
    }

    // 7. Response status codes
    println!("\n7. Common response status codes...");
    let statuses = [
        (200, "OK"),
        (201, "Created"),
        (400, "Bad Request"),
        (401, "Unauthorized"),
        (404, "Not Found"),
        (500, "Internal Server Error"),
    ];

    for (code, text) in &statuses {
        println!("   {} {}", code, text);
    }

    // 8. Clear routes
    println!("\n8. Clearing routes...");
    interceptor.clear_routes();
    println!("   Routes cleared. Count: {}", interceptor.route_count());

    println!("\n✅ Network interception example completed!");
    Ok(())
}
