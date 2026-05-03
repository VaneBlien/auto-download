
# auto-download

A concurrent downloader in Rust with resume support, built around an event-driven state machine.

## Design

Each download is modeled as an **event** with its own state machine:

```
Pending → Downloading → Paused
  ↓                      ↓
  ↓        Downloading ←─┘
  ↓            ↓
  └──────→ Completed
             ↑
  Failed ────┘ (retry)
```

- **Event**: A download task containing URL, destination path, byte range, retry count, and state.
- **Event Queue**: An `mpsc` channel feeds events to worker tasks.
- **Worker Pool**: N async tasks (tokio) pull events from the queue, drive the state machine, and report progress back.
- **Resume**: Uses HTTP Range headers (`reqwest`) to continue from the last downloaded byte saved in a `.part` file.
- **Progress**: Real-time progress bars via `indicatif`.

## Features

- [x] Single-file download with progress bars
- [x] Resume interrupted downloads (HTTP Range)
- [x] Concurrent worker pool (tokio async)
- [x] CLI via `clap` (URLs, thread count, output directory)
- [x] Retry on failure (configurable max retries)
- [x] Real-time progress bars with `indicatif`

## Quick Start

```bash
# Clone and build
git clone <your-repo-url>
cd auto-download
cargo build --release

# Download a single file
./target/release/auto-download https://proof.ovh.net/files/1Mb.dat

# Download multiple files with custom threads and output directory
./target/release/auto-download -t 4 -o downloads \
  https://proof.ovh.net/files/10Mb.dat \
  https://proof.ovh.net/files/100Mb.dat
```

## CLI Options

```
Usage: auto-download [OPTIONS] <URLS>...

Arguments:
  <URLS>...  One or more URLs to download

Options:
  -t, --threads <THREADS>        Number of worker tasks [default: 4]
  -o, --output-dir <OUTPUT_DIR>  Output directory [default: .]
  -h, --help                     Print help
```

## How Resume Works

When a download is interrupted (Ctrl+C or network error), a `.part` file is left behind. On the next run with the same URL and output path, the downloader:

1. Detects the existing `.part` file
2. Reads its size to determine how many bytes have been downloaded
3. Sends an HTTP `Range: bytes=<downloaded>-` header
4. Appends new data to the `.part` file
5. Renames `.part` to the final filename upon completion

## Project Structure

```
src/
├── main.rs       # CLI parsing, task dispatch, result collection
├── event.rs      # DownloadEvent struct and DownloadState enum
├── downloader.rs # HTTP download logic with Range support
├── worker.rs     # Async worker loop with retry logic
├── reporter.rs   # Progress bar management via indicatif
```

## Dependencies

- `reqwest` - HTTP client with async and streaming support
- `clap` - Command-line argument parsing
- `indicatif` - Real-time progress bars
- `tokio` - Async runtime
- `futures-util` - Stream extension traits

## License

MIT


