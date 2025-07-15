# ADR-0015: Caching Strategy

## Status

Accepted

## Context

Union Square can benefit from caching LLM responses to:
1. Reduce costs by avoiding duplicate API calls
2. Improve response times for repeated queries
3. Enable offline testing with cached responses
4. Support development/testing environments

However, caching must:
- Not impact the <5ms latency requirement
- Be transparent to applications
- Respect cache control headers
- Support cache invalidation
- Handle streaming responses appropriately

LLM responses present unique caching challenges:
- Non-deterministic responses (temperature > 0)
- Large response sizes
- Model version sensitivity
- Context-dependent outputs

## Decision

We will implement an optional, configurable caching layer with multiple strategies:

### Cache Key Generation

```rust
struct CacheKey {
    provider: ProviderId,
    model: String,
    request_hash: [u8; 32],  // SHA256 of canonical request
    cache_version: u32,      // For cache invalidation
}

impl CacheKey {
    fn from_request(provider: &ProviderId, request: &Request<Body>) -> Self {
        // Canonicalize request for consistent hashing
        let canonical = canonicalize_request(request);
        let hash = sha256(&canonical);
        
        Self {
            provider: provider.clone(),
            model: extract_model(request),
            request_hash: hash,
            cache_version: CURRENT_CACHE_VERSION,
        }
    }
}

fn canonicalize_request(request: &Request<Body>) -> Vec<u8> {
    // Sort JSON keys, normalize whitespace, etc.
    // Exclude headers that don't affect response
}
```

### Cache Storage Tiers

1. **Hot Cache (In-Memory)**
   - Recent/frequent responses
   - Fixed size with LRU eviction
   - Sub-millisecond access

2. **Warm Cache (PostgreSQL)**
   - Larger capacity
   - Persistent across restarts
   - ~10ms access

### Cache Configuration

```toml
[cache]
enabled = false  # Opt-in per application

[cache.policy]
default_ttl_seconds = 3600
max_ttl_seconds = 86400
respect_cache_control = true
cache_non_deterministic = false  # Only cache temperature=0

[cache.storage]
memory_size_mb = 512
database_size_gb = 10
compression = "zstd"

[cache.rules]
# Per-model overrides
"gpt-4" = { ttl = 7200, enabled = true }
"claude-3" = { ttl = 3600, enabled = true }

# Pattern-based rules
exclude_patterns = ["*/stream", "*/embeddings"]
```

### Cache Headers

Support standard HTTP cache headers plus custom ones:

```
# Request headers (from client)
Cache-Control: no-cache              # Bypass cache
Cache-Control: max-age=3600          # Use cache if fresher than 1 hour
X-UnionSquare-Cache-Key: "custom"    # Additional cache key component

# Response headers (to client)
X-Cache: HIT|MISS|BYPASS
X-Cache-Key: "sha256:abcd..."
Age: 120                             # Age of cached response
```

### Cache Lookup Flow

```rust
async fn handle_request(request: Request<Body>) -> Response<Body> {
    let cache_control = parse_cache_control(&request);
    
    // 1. Check if caching is enabled and allowed
    if !cache_config.enabled || cache_control.no_cache {
        return forward_request(request).await;
    }
    
    // 2. Generate cache key
    let key = CacheKey::from_request(&provider, &request);
    
    // 3. Try hot cache first
    if let Some(cached) = hot_cache.get(&key).await {
        if !is_stale(&cached, &cache_control) {
            return build_cached_response(cached, CacheHit::Hot);
        }
    }
    
    // 4. Try warm cache
    if let Some(cached) = warm_cache.get(&key).await {
        if !is_stale(&cached, &cache_control) {
            // Promote to hot cache
            hot_cache.put(&key, &cached).await;
            return build_cached_response(cached, CacheHit::Warm);
        }
    }
    
    // 5. Cache miss - forward request
    let response = forward_request(request).await;
    
    // 6. Store in cache if appropriate
    if should_cache(&response) {
        let entry = CacheEntry::new(response.clone());
        hot_cache.put(&key, &entry).await;
        warm_cache.put(&key, &entry).await;
    }
    
    response
}
```

### Streaming Response Handling

```rust
enum CachedResponse {
    Complete(Bytes),
    Streamed {
        chunks: Vec<Bytes>,
        timing: Vec<Duration>,  // Original chunk timing
    },
}

// For cached streaming responses, replay with original timing
async fn replay_stream(cached: &CachedResponse) -> impl Stream<Item = Bytes> {
    match cached {
        CachedResponse::Streamed { chunks, timing } => {
            stream::iter(chunks.iter().zip(timing.iter()))
                .then(|(chunk, delay)| async move {
                    tokio::time::sleep(*delay).await;
                    chunk.clone()
                })
        }
        CachedResponse::Complete(data) => {
            stream::once(async move { data.clone() })
        }
    }
}
```

### Cache Invalidation

```rust
// API endpoints for cache management
POST   /api/v1/cache/invalidate/pattern
POST   /api/v1/cache/invalidate/model/{model}
POST   /api/v1/cache/clear
GET    /api/v1/cache/stats

// Automatic invalidation
- Model version changes
- TTL expiration
- LRU eviction
- Manual invalidation via API
```

## Consequences

### Positive

- Significant cost savings for repeated queries
- Improved response times for cached hits
- Transparent to client applications
- Flexible configuration per use case
- Useful for testing and development

### Negative

- Cache storage overhead
- Cache key computation cost
- Potential for stale responses
- Complexity in streaming replay
- Cache coherency challenges

### Mitigation Strategies

1. **Performance**: Async cache lookups, don't block hot path
2. **Staleness**: Conservative TTLs, clear invalidation APIs
3. **Storage**: Compression, tiered storage, automatic cleanup
4. **Monitoring**: Cache hit rates, latency impact metrics
5. **Testing**: Cache-specific test cases

## Alternatives Considered

1. **No Caching**
   - Simpler, always fresh
   - Rejected: Missing cost optimization opportunity

2. **External Cache (Redis)**
   - Standard caching solution
   - Rejected: Additional operational complexity

3. **CDN-Style Caching**
   - Edge caching approach
   - Rejected: Doesn't fit LLM use case well

4. **Application-Level Caching**
   - Let applications handle caching
   - Rejected: Duplicated effort, no shared benefit

5. **Response-Hash Based**
   - Cache based on response content
   - Rejected: Must know response to cache it

## Implementation Notes

- Cache warming for common queries
- Consider cache stampede protection
- Monitor cache efficiency closely
- Document cache behavior clearly
- Support cache debugging headers

## Related Decisions

- ADR-0008: Dual-path Architecture (caching in hot path)
- ADR-0010: Tiered Projection Strategy (cache storage tiers)
- ADR-0011: Provider Abstraction (per-provider cache rules)