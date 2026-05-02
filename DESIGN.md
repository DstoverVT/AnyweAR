# AnyweAR Design Overview

## Purpose

AnyweAR is intended to be a wearable memory and activity tracking system.

At a high level, the system captures first-person video from a wearable device, relays that video through a phone, and runs heavier analysis in the cloud. The goal is not just to save video, but to convert daily life into searchable, structured memories that can help a user understand patterns, answer questions about the past, and track specific behaviors over time.

This repository exists to build that pipeline. The long-term product is a full wearable-plus-phone-plus-cloud system. The current codebase is an early implementation of the ingestion and memory extraction architecture that this document describes.

## Product Vision

The envisioned device is a lightweight wearable camera that can observe what the user sees throughout the day. The device itself should stay relatively simple and power-efficient. It records video, chunks it into manageable segments, and hands those segments off to the phone. The phone acts as the user's local hub, buffering data, handling connectivity transitions, and showing results in an app. The cloud does the expensive work: interpreting video, extracting events, storing memories, generating summaries, and answering user questions.

A key design principle is that the wearable does not need to do sophisticated on-device inference. This keeps the hardware simpler and makes it easier to improve the system over time by upgrading server-side models instead of constantly redesigning the wearable. Real-time output is helpful in some cases, but it is not a requirement for the initial product. The more important goal is reliable background processing and a good app experience once results are ready.

## Core User Experience

The user wears a camera-based device during normal daily life. The system captures periodic video chunks and moves them to the phone whenever possible. The phone stores them locally until it can upload them to the cloud. Once uploaded, backend processing turns raw footage into structured observations, higher-level events, and user-facing summaries.

The user primarily interacts with the system through the phone app rather than through the wearable. In the app, the user can:

- tell the system what they care about tracking
- review extracted events and daily summaries
- log symptoms or outcome metrics manually
- view trends over days, weeks, and months
- ask questions about the past

The wearable is therefore mostly an input sensor. The phone app is the main user interface, and the cloud is the main intelligence layer.

## Example Use Cases

### Food and outcome tracking

One major use case is passive food logging. The system should detect when the user eats, infer what food or drink is involved, and keep a timeline of intake events. The user can then log outcome metrics such as stomach pain, bloating, energy level, gym performance, sleep quality, or any other metric they care about. Over time, the app can look for correlations between foods and those outcomes.

This is meant to be flexible rather than limited to calorie tracking. The important idea is that the system automatically captures intake events and lets the user define which downstream metrics matter.

### Habit tracking and reduction

Another use case is tracking unwanted habits such as smoking or drinking alcohol. The user can tell the system which habits they want monitored. The backend can then look for evidence of those activities and maintain a long-term log of frequency, timing, and trends. The app can present counts per day, week, or month and help the user understand whether they are improving.

### Automatic life journaling

The system should also support a broader "daily journal" mode. Even when the user is not tracking one specific behavior, the system can summarize the day into meaningful activities, locations, and notable events. This is useful both for reflection and for creating a searchable memory record of daily life.

### Memory retrieval and question answering

A long-term goal is the ability to ask practical questions about the past, such as:

- "Where are my keys?"
- "Did I take out the trash yesterday?"
- "When was the last time I drank alcohol?"
- "What did I eat before my stomach hurt?"

This requires more than simple video storage. The system needs structured memory records, timestamps, searchable text, and vector-based retrieval so that queries can be answered from past observations and events.

## Product Requirements

### Wearable operation

The wearable should work anywhere, including in situations without Wi-Fi internet access. It does not need to be independently connected to the internet as long as it can continue recording and eventually transfer data to the phone.

### Deferred processing is acceptable

The system does not need to provide immediate, real-time responses for the initial design. It is acceptable for footage to be processed later in the background and for results to appear in the app after some delay. This is an intentional tradeoff that allows the system to use more sophisticated algorithms than would be practical on-device.

### Phone-first user interface

The phone app is the canonical interface for configuration, review, and querying. The user should not need a complex interface on the wearable. An iPhone-first implementation is a reasonable initial scope reduction, with Android support considered later.

## System Architecture

The intended architecture has three main runtime components:

1. Wearable device
2. Phone app
3. Cloud backend

Each one has a different role.

### 1. Wearable device

The wearable is responsible for capturing egocentric video and packaging it for transfer. A simple baseline design is to record fixed-size video chunks, currently imagined as `30-second` segments. That duration is not fundamental and may change later for latency, battery, storage, or modeling reasons, but it is a useful starting point.

The wearable should:

- record chunked video continuously or semi-continuously
- assign sequence numbers to chunks
- store chunks locally until transferred
- signal to the phone when a chunk is ready
- transfer the chunk media to the phone reliably

The current design idea is:

- use Bluetooth as a low-bandwidth control channel
- use Wi-Fi Direct as the higher-bandwidth data channel for video transfer

Bluetooth can be used for coordination such as "chunk ready" notifications, availability, and retransmission requests. Wi-Fi Direct can carry the actual video file transfer. Sequence numbers matter because they let the phone detect missing data and request retransmission. This is important because chunk order defines later server-side analysis windows.

The wearable likely needs:

- a camera
- local compute sufficient for recording and transfer logic
- Bluetooth
- Wi-Fi
- local flash storage or equivalent non-volatile storage
- a battery sized for day-scale usage

In the near term, the wearable does not need to run the heavy AI stack locally. Its job is dependable capture and delivery.

### 2. Phone app

The phone is the bridge between the wearable and the cloud. It provides a friendlier interface, more reliable networking, local buffering, and a place to show results.

Phone responsibilities include:

- receiving video chunks from the wearable
- keeping chunks in local app storage until upload succeeds
- uploading chunks to cloud storage or cloud services
- retrying when connectivity is poor
- allowing uploads on Wi-Fi immediately
- optionally allowing uploads on cellular when the user opts in
- keeping local copies until the cloud confirms receipt
- displaying processed results to the user
- collecting user-entered metrics, feedback, and tracker preferences

This buffering layer is important because the system should still work when the user is offline, away from home, or temporarily disconnected from the internet. The phone can accumulate footage and synchronize later.

The app is also where user intent enters the system. The user should be able to specify what they want tracked, for example:

- foods eaten
- alcohol usage
- smoking
- key or wallet placement
- custom symptoms or performance metrics

Those preferences can then influence downstream model routing and correlation logic.

### 3. Cloud backend

The cloud backend performs durable storage, heavy inference, memory extraction, retrieval, and analytics. This is where raw media is turned into structured facts that can later support summaries, insights, and search.

High-level backend responsibilities include:

- storing or registering uploaded video
- grouping incoming chunks into analysis windows
- running general-purpose understanding over the footage
- routing likely important segments into more specialized processing
- storing structured observations and events
- generating summaries and journals
- supporting retrieval and question answering
- correlating events with user-entered metrics

## Processing Pipeline

The target processing pipeline is layered.

### Step 1: Video chunk ingestion

Raw footage arrives from the phone as chunked video with timestamps and sequence numbers.

The current prototype in this repository already uses `30-second` chunks as the ingestion unit.

### Step 2: Analysis window construction

Rather than analyzing every individual chunk in isolation, the backend groups consecutive chunks into larger windows that give the model more context. The current implementation groups `4` consecutive `30-second` chunks into a `2-minute` analysis window.

This windowed approach has several advantages:

- it provides more temporal context than a single short clip
- it reduces per-request overhead
- it makes journaling and event extraction more coherent
- it allows chunk-level transfer reliability while preserving longer-range reasoning

### Step 3: General understanding layer

The first inference layer should try to broadly understand what is happening in the window without overfitting to one niche task. This "general layer" supports daily journaling and also provides routing hints for later specialized models.

Examples of information this layer should extract:

- activity
- location or environment context
- objects in use or nearby
- rough event categories
- start and end timestamps
- a natural-language description
- search-friendly text
- likely trigger candidates for specific layer

Examples of broad event types might include:

- meal event
- commute
- exercise
- household chore
- smoking event
- drinking event
- social interaction
- work session

This layer should answer the question: "What generally happened here?"

### Step 4: Specific follow-up layer

The second inference layer is narrower and more targeted. It should run only when either:

- the user has asked to track a specific thing
- the general layer indicates that a chunk or window is a good candidate

This layer is responsible for higher-confidence, higher-specificity interpretation. For example:

- identifying specific food items

One possible pattern is:

1. Use a lighter object or action detector to find likely regions or frames of interest.
2. Crop or isolate those regions.
3. Send the focused evidence to a stronger multimodal model for fine-grained interpretation.

Potential model ideas discussed so far include:

- a detector such as YOLO-family models for object proposals or regions of interest
- a multimodal model such as Gemini Flash or GPT-class vision models for semantic interpretation

This layer should answer the question: "What exactly was this thing or action?"

### Step 5: Memory construction and storage

After extraction, the system should convert model output into durable records that can be searched, aggregated, and displayed. The system should not rely only on free-form text summaries. It should store structured facts alongside summaries and embeddings.

Examples of memory content to preserve:

- timestamps
- event type
- objects
- actions
- location context
- generated description
- confidence
- source video evidence
- links to user-defined trackers

The long-term design is to treat the cloud memory store as a structured, queryable representation of the user's past rather than just a bucket of video files.

### Step 6: Retrieval, analytics, and app presentation

Once memories are stored, the app can present them in several ways:

- chronological event feed
- daily summaries
- tracker dashboards
- correlation insights
- question answering over past activity

This is where the product becomes useful to the end user. The backend should support both exact filtering and fuzzy semantic search.

## Data Model Direction

The long-term storage layer will likely use PostgreSQL, with `pgvector` for embeddings and semantic retrieval.

Candidate relational tables include:

- `users`
- `video_chunks`
- `observations`
- `events`
- `memory_records`
- `user_metrics`
- `symptom_logs`
- `tracker_configs`
- `user_feedback`

Examples of embedding-bearing fields include:

- memory embedding
- observation embedding
- evidence-frame embedding

This supports two complementary access patterns:

- structured querying for exact fields such as dates, event types, tracker IDs, and numeric metrics
- embedding search for fuzzy questions and memory retrieval

## Query and Memory Retrieval

One of the most important product goals is answering user questions about the past. A likely retrieval flow is:

1. Interpret the user's question.
2. Narrow the search space using structured filters when possible.
3. Search the memory store using embeddings for semantic recall.
4. Pull back the most relevant observations, events, and evidence.
5. Synthesize an answer for the user, ideally with references to when or where the evidence came from.

For example, a question like "Did I take out the trash yesterday?" could be broken into:

- time range: yesterday
- action concept: taking out trash
- possible location context: home, curb, outside bin

Similarly, "Where are my keys?" may rely on the latest high-confidence observation related to keys plus the surrounding location context and the last known placement event.

## Correlation and Insight Generation

For food and habit tracking, the system should do more than list events. It should connect them to outcomes the user cares about.

Examples:

- correlate food intake with stomach pain ratings
- correlate alcohol usage with weight, sleep, or workouts
- correlate smoking frequency with self-reported symptoms

The app should let users define custom outcome metrics, including numeric ratings such as pain from `1-10`, performance metrics such as gym PRs, or other repeating logs. The backend can then recompute correlations when new metric entries arrive.

Possible first-pass analytics include:

- rolling averages over recent windows such as `7 days`
- comparisons between days with and without a tracked behavior
- simple lag-based correlation heuristics between intake and later symptoms

An LLM can help explain these patterns in natural language, but the underlying structured calculations should be preserved so the app is not purely narrative.

## Design Principles

Several principles shape the architecture:

### Keep the wearable simple

The wearable should focus on capture, buffering, and transfer. Expensive inference belongs on the phone or cloud whenever possible.

### Make offline capture acceptable

The system should degrade gracefully when internet is unavailable. Recording and local buffering should continue even when upload is delayed.

### Store structured memory, not just summaries

Natural-language summaries are useful, but durable product value comes from structured events, timestamps, entities, and evidence links that support search and analytics.

### Use user intent to focus compute

The user should be able to tell the system what matters. That can guide model routing, reduce unnecessary processing, and improve usefulness.

### Prefer layered inference over one monolithic pass

A broad general layer plus narrower specific follow-up models should be easier to scale, debug, and improve than a single model trying to solve everything at once.
