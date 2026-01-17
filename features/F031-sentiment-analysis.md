# F031: Sentiment & Emotional Analysis

| Status | Priority | Crate |
|--------|----------|-------|
| `complete` | medium | aegis-core |

## Description

Add local sentiment and emotional tone analysis to flag content for parental review. Unlike blocking (F004), this feature identifies concerning emotional patterns that may warrant parental awareness without blocking the interaction.

**Use Cases:**
- Child expressing feelings of hopelessness or loneliness to AI
- Conversations about bullying, peer pressure, or relationship issues
- Sustained negative sentiment patterns over time
- Crisis indicators that complement existing SelfHarm category

**Privacy-First:** All analysis runs locally using lexicon-based detection. No cloud services, no data transmission.

## Dependencies

- **Requires**: F004 (Tiered Classification), F008 (SQLite Storage), F012 (Parent Dashboard)
- **Blocks**: None

## Technical Approach

### Tier 3: Sentiment Analysis

Runs after Tier 1 (Keywords) and Tier 2 (ML). Does not block - only flags for review.

```
Input Text → Sentiment Lexicons → Emotion Scores → Flag Decision → Store Event
```

### Sentiment Categories

| Category | Indicators | Example Patterns |
|----------|------------|------------------|
| Distress | Sadness, hopelessness, anxiety | "I feel so alone", "nobody cares" |
| CrisisIndicator | Self-harm adjacent, suicidal ideation | "I don't want to be here anymore" |
| Bullying | Peer conflict, harassment discussion | "they keep making fun of me" |
| NegativeSentiment | Sustained negativity, anger | High negative word density |

### New Structures

```rust
pub enum SentimentFlag {
    Distress,
    CrisisIndicator,
    Bullying,
    NegativeSentiment,
}

pub struct SentimentResult {
    pub flags: Vec<SentimentMatch>,
    pub overall_sentiment: f32,  // -1.0 (negative) to 1.0 (positive)
    pub duration_us: u64,
}

pub struct SentimentMatch {
    pub flag: SentimentFlag,
    pub confidence: f32,
    pub matched_phrases: Vec<String>,
}

pub struct FlaggedEvent {
    pub id: i64,
    pub profile_id: i64,
    pub timestamp: DateTime<Utc>,
    pub flag_type: SentimentFlag,
    pub confidence: f32,
    pub content_snippet: String,  // First 200 chars, redacted
    pub source: String,           // Which AI service
    pub acknowledged: bool,
    pub acknowledged_at: Option<DateTime<Utc>>,
}
```

### Lexicon Design

VADER-inspired approach with domain-specific extensions:

1. **Base Sentiment Lexicon**: ~7500 words with valence scores
2. **Emotion Lexicons**: Specific word lists per category
3. **Intensifiers**: "very", "extremely" → boost scores
4. **Negations**: "not sad" → invert sentiment
5. **Phrase Patterns**: Multi-word expressions ("I feel like giving up")

### Storage Schema

```sql
CREATE TABLE flagged_events (
    id INTEGER PRIMARY KEY,
    profile_id INTEGER NOT NULL,
    timestamp TEXT NOT NULL,
    flag_type TEXT NOT NULL,
    confidence REAL NOT NULL,
    content_snippet TEXT NOT NULL,
    source TEXT,
    acknowledged INTEGER DEFAULT 0,
    acknowledged_at TEXT,
    FOREIGN KEY (profile_id) REFERENCES profiles(id)
);

CREATE INDEX idx_flagged_profile ON flagged_events(profile_id);
CREATE INDEX idx_flagged_timestamp ON flagged_events(timestamp);
CREATE INDEX idx_flagged_unacknowledged ON flagged_events(acknowledged) WHERE acknowledged = 0;
```

### Profile Configuration

```rust
pub struct SentimentConfig {
    pub enabled: bool,
    pub sensitivity: SentimentSensitivity,  // Low, Medium, High
    pub enabled_categories: HashSet<SentimentFlag>,
    pub notify_on_flag: bool,
}
```

## Acceptance Criteria

### Core Analyzer
- [x] SentimentAnalyzer implements lexicon-based detection
- [x] Supports all 4 sentiment categories
- [x] <10ms latency for analysis
- [x] Handles negation and intensifiers
- [x] Confidence scores 0.0-1.0

### Integration
- [x] Runs as Tier 3 after ML classification
- [x] Does not affect blocking decisions
- [x] Stores flagged events in SQLite
- [x] Respects profile-level settings

### Parent Dashboard
- [x] "Flagged Items" view shows all flags
- [x] Filterable by profile, category, date
- [x] Acknowledge/dismiss functionality
- [x] Shows content snippet with context
- [ ] Notification for new flags (if enabled) - Future enhancement

### API Endpoints
- [x] GET /api/flagged - List flagged events
- [x] POST /api/flagged/:id/acknowledge - Acknowledge flag
- [x] GET /api/flagged/stats - Summary statistics

### Testing
- [x] Unit tests for each sentiment category
- [x] False positive rate <5% on benign content
- [x] Detection rate >80% on test corpus
- [x] Integration tests with classification pipeline

## Performance Targets

| Operation | Target |
|-----------|--------|
| Sentiment analysis | <10ms |
| Flag storage | <5ms |
| Dashboard load | <200ms |

## Privacy Considerations

- Content snippets limited to 200 characters
- No full conversation storage
- Flagged events deletable by parent
- All processing local (no cloud)
- Sensitive phrases redacted in snippets

## Future Enhancements

1. **ONNX Emotion Model**: Add ML-based emotion detection for higher accuracy
2. **Trend Analysis**: Detect patterns over time (sustained negativity)
3. **Conversation Context**: Multi-turn sentiment tracking
4. **Custom Lexicons**: Parent-defined concerning phrases

## Notes

This feature focuses on awareness, not surveillance. The goal is to help parents identify when their child may need support, not to spy on conversations. UI should emphasize supportive framing.
