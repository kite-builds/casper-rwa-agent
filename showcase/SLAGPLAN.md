# Casper Builder Spotlight — Battle Plan (CasperRWA-Agent)

**Event:** Casper Agentic Buildathon — Builder Spotlight (X Spaces + YouTube)
**When:** Saturday 2026-06-27, **09:00 GMT = 11:00 Oslo (CEST)**
**Slot:** 6 minutes live · our project = CasperRWA-Agent (BUIDL #44481)
**Links:** X Spaces https://twitter.com/i/spaces/1AxRnnEzMYkxl · YouTube https://www.youtube.com/live/tJ0Qp2qW9a8

---

## Strategy: maximize autonomy, minimize what depends on SP

The whole slot is delivered by **one self-contained video** (slides baked in + the agent
narrating itself). The organizer plays it during our slot. That removes every blocker:
no X-speaker account needed, no live-audio routing, no manual slide-cueing.

**Assets (all built, in `casper-rwa-agent/showcase/`):**
- `CasperRWA-Agent_Showcase.mp4` — 4:00, 720p, narrated, slides baked in ← the deliverable
- `CasperRWA-Agent_Showcase.pdf` — 9-slide deck (backup if they want to drive slides manually)
- `deck.html` — source deck (live-navigable)
- `QA_PACK.md` — pre-written answers for the combined Q&A
- narration/ + audio/ + slides/ — raw assets to regenerate/edit

## Path A — RECORDED (primary, ~zero SP dependency)
1. SP forwards the organizer the video + the short message (below). **← only SP action.**
2. Organizer confirms they'll play the MP4 in our slot. (Slides are in the video, so they
   don't even need the PDF or cueing.)
3. At 11:00 Oslo Sat: organizer plays it. We monitor the stream.
4. Combined Q&A at the end: if we get a question, answer from `QA_PACK.md` (SP reads it, or
   I draft a fresh reply in seconds and SP relays). No live mic needed.

## Path B — LIVE (fallback, only if organizer requires live speaking)
- Needs: an X account SP controls, promoted to speaker + SP reachable 11:00 Oslo.
- I install BlackHole (virtual mic) on the mini tonight, route TTS into the Space, and play
  the same narration live while cueing the organizer per slide. Higher risk; only if forced.

## Open dependency (the one thing I need from SP)
- **The organizer's channel** (X DM / Telegram / email) to deliver the video + message.
  Easiest: SP forwards the drafted message + MP4 on whatever thread the Q&A reply came from.
- If recorded is rejected → switch to Path B (needs the X account).

## Timeline
- **Fri evening:** ✅ deck, ✅ video, ✅ Q&A pack, ✅ reminders set (T-2h + T-15). Send video to organizer.
- **Sat 09:00 Oslo:** prep ping (cron) — confirm organizer has the file + will play it.
- **Sat 10:45 Oslo:** go-live ping (cron). Monitor X Spaces + YouTube.
- **Sat ~11:00:** our 6-min plays. Stand by for Q&A.
