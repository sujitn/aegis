# F033: Image Content Filtering

| Status | Priority | Crate |
|--------|----------|-------|
| `complete` | high | aegis-core, aegis-proxy |

## Description

Filter explicit/NSFW content in AI image generation workflows. Covers prompt filtering (outbound requests), image response filtering (generated images), and image upload filtering (img2img, inpainting). Uses tiered classification: keyword patterns for prompts, ONNX-based NSFW classifier for images.

## Dependencies

- **Requires**: F004 (Tiered Classification), F016 (MITM Proxy), F027 (Dynamic Site Registry), F019 (User Profiles)
- **Blocks**: None

## Scope

### In Scope

- Block explicit image generation prompts (text filtering)
- Block NSFW generated images (binary/base64 response filtering)
- Block NSFW image uploads (multipart form-data filtering)
- Support major image generation services:
  - xAI Grok Aurora (`api.x.ai`, `grok.x.ai`)
  - OpenAI DALL-E / GPT-Image (`api.openai.com`)
  - Stability AI (`api.stability.ai`)
  - Leonardo.ai (`cloud.leonardo.ai`)
  - Ideogram (`api.ideogram.ai`)
  - Runway ML (`api.runwayml.com`)
  - Black Forest Labs Flux (`api.bfl.ml`, `api.together.xyz`)
  - Replicate (`api.replicate.com`)

### Out of Scope

- Discord-based services (Midjourney) - no API to intercept
- Video content analysis (future feature)
- Real-time streaming preview filtering (complex, defer)

## Acceptance Criteria

### Image Generation Domain Registry

- [ ] Add `image_gen` category to SiteRegistry alongside existing `consumer`/`api`/`enterprise`
- [ ] Register bundled image generation domains with `image_gen` category
- [ ] Parent can add custom image generation domains via dashboard
- [ ] Distinguish between text LLM domains and image generation domains

### Prompt Filtering (Outbound)

- [ ] Detect explicit image generation prompts using keyword patterns
- [ ] Add image-specific explicit terms to community rules (nudity, pornographic, NSFW art terms)
- [ ] Integrate with existing F004 tiered classification pipeline
- [ ] Block requests with explicit prompts to image generation domains
- [ ] Log blocked prompts with `image_gen_prompt` action type

### Image Response Filtering (Inbound)

- [ ] Intercept HTTP responses from registered image generation domains
- [ ] Extract images from JSON responses (base64 `data:image/...` or `b64_json` fields)
- [ ] Extract images from binary responses (`Content-Type: image/*`)
- [ ] Load ONNX NSFW classifier on first image intercept (lazy loading)
- [ ] Classify extracted images and block if score exceeds profile threshold
- [ ] Return block page/error response for blocked images
- [ ] Log blocked images with classification score (hash only, not image data)

### Image Upload Filtering

- [ ] Intercept `multipart/form-data` POST requests to image generation domains
- [ ] Extract uploaded images from form data
- [ ] Classify images before forwarding request
- [ ] Block uploads that exceed profile NSFW threshold
- [ ] Support common image formats: JPEG, PNG, WebP, GIF (first frame)

### NSFW Classifier

- [ ] Use Falconsai/nsfw_image_detection model (ViT-based, ONNX export)
- [ ] Model input: 224x224 RGB images
- [ ] Model output: binary classification (normal/nsfw) with confidence score
- [ ] Classification target: <100ms on average CPU
- [ ] Graceful fallback if model file missing (log warning, allow through)
- [ ] Configurable model path via settings

### Profile Integration

- [ ] Per-profile NSFW threshold setting (0.0 - 1.0)
- [ ] Default thresholds by age group:
  - Child (< 13): 0.3 (aggressive blocking)
  - Teen (13-17): 0.5 (balanced)
  - Adult (18+): 0.8 (permissive)
- [ ] Parent can customize threshold per profile
- [ ] Threshold applies to both response and upload filtering

### Performance

- [ ] Only classify images from registered `image_gen` domains
- [ ] Skip classification for images larger than configurable max size (default 10MB)
- [ ] Cache model in memory after first load
- [ ] Resize images to 224x224 before classification (required by model)
- [ ] Log classification duration for monitoring

### Privacy

- [ ] Never store image data, only classification result
- [ ] Store SHA-256 hash of blocked images for audit trail
- [ ] All classification happens locally, no external API calls
- [ ] Clear any temporary image buffers after classification

## Notes

### Recommended ONNX Model

**Falconsai/nsfw_image_detection** (Hugging Face)
- Architecture: Vision Transformer (ViT) fine-tuned on ImageNet-21k
- Input: 224x224 RGB images
- Output: Binary classification (normal/nsfw)
- ~80M downloads, production-proven
- Apache 2.0 compatible license
- Can be exported to ONNX using `optimum` library

### Image Extraction Patterns

```
# JSON response patterns
{"data": [{"b64_json": "..."}]}           # OpenAI
{"artifacts": [{"base64": "..."}]}         # Stability AI
{"output": ["data:image/png;base64,..."]}  # Replicate
{"generations": [{"url": "..."}]}          # Leonardo.ai
{"images": [{"image": "..."}]}             # xAI Grok
```

### Domain Categories

| Domain | Category | Notes |
|--------|----------|-------|
| api.x.ai | image_gen | Grok Aurora |
| api.openai.com | image_gen + api | DALL-E, GPT-Image |
| api.stability.ai | image_gen | Stable Diffusion |
| cloud.leonardo.ai | image_gen | Leonardo.ai |
| api.ideogram.ai | image_gen | Ideogram |
| api.runwayml.com | image_gen | Runway (video/image) |
| api.bfl.ml | image_gen | Black Forest Labs Flux |
| api.together.xyz | image_gen | Together AI (hosts Flux) |
| api.replicate.com | image_gen | Replicate (hosts many models) |

### Crate Organization

- `aegis-core/src/classifier/image.rs` - NSFW image classifier
- `aegis-core/src/site_registry.rs` - Add `image_gen` category
- `aegis-proxy/src/image_extractor.rs` - Extract images from responses
- `aegis-proxy/src/handler.rs` - Response interception hooks
