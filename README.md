# Chess Binary Format (`cbin`)

Chess Binary Format (`cbin`) is a binary format for archival storage and fast search of large amounts of chess games. It uses [FlatBuffers](https://flatbuffers.dev/) to store data, allowing libraries to be generated automatically and data to be read with zero-copy access.

## Features

- **Compact binary format:** uncompressed `cbin` archives are smaller than zstd-compressed PGN archives. Data like strings, moves, etc. is deduplicated inside the archive.
- **Zero-parsing, zero-copy access:** Thanks to FlatBuffers, `cbin` archives can be read straight from the disk with no extra memory usage or CPU overhead. They can be memory-mapped for even faster access.
- **Parallel processing:** `cbin` archives store games in a series of blocks, allowing for efficient parallel processing of games. PGN archives cannot do this because they require parsing.
- **Dead-simple implementation:** FlatBuffers generates libraries for most languages out of the box. The only custom code you need to write is to do simple prefix length calculations.

## Usage

This format is still in its early stages. Please see the [schema/chess.fbs](schema/chess.fbs) for the current FlatBuffers schema.
