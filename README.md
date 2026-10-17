# Download manager


Multi-conneciton download manager.  Downloads one file with multiple simaltaneous connections by requesting the file in chunks with HTTP Range requests.

Currently supports:
 * Multiple download mirrors/sources for the same file
 * Automatic recovery from network interruption
 * Consumer API - clients can implement a custom [file consumer](dlmgr/src/api/sequential_chunk_consumer.rs) and the library will stream the downloaded data re-assembled into the correct order.  This can be used to support saving the downloaded files to arbitrary storage backends
 * Optional fallback to non-parallel download if URL does not support range requests


Planned / WIP
 * Support for automatic resumption from process interruption
 * Handling of redirects
 * Support for downloading a pre-split file based on a chunk index

