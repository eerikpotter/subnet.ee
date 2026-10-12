// Count decoded package bytes (not compressed Content-Length). Compile as bytes
// arrive, but verify the build's SHA-384 before returning a module for execution.
export async function loadCraftWasm(url, total, integrity) {
  let loaded = 0;
  let lastReport = -Infinity;
  let reader;
  const report = (phase, force = false) => {
    const now = performance.now();
    if (!force && now - lastReport < 80 && loaded !== total) return;
    document.documentElement.setAttribute("data-craft-download", JSON.stringify({ phase, loaded, total }));
    lastReport = now;
  };
  try {
    if (!Number.isSafeInteger(total) || total <= 0 || !/^sha384-[A-Za-z0-9+/]{64}$/.test(integrity)) {
      throw new Error("Invalid Wasm build metadata");
    }
    report("download", true);
    // Keep the original versioned URL and ordinary HTTP-cache semantics. A
    // fetch integrity option buffers the whole body, hiding download progress;
    // instead the equivalent digest check below gates stream completion.
    const response = await fetch(url);
    if (!response.ok || !response.body) throw new Error(`Wasm request failed: ${response.status}`);
    reader = response.body.getReader();
    let bytes = new Uint8Array(total);
    const stream = new ReadableStream({
      async pull(controller) {
        try {
          const { done, value } = await reader.read();
          if (done) {
            if (loaded !== total) throw new Error("Incomplete Wasm package");
            report("verify", true);
            const digest = new Uint8Array(await crypto.subtle.digest("SHA-384", bytes));
            const actual = "sha384-" + btoa(String.fromCharCode(...digest));
            if (actual !== integrity) throw new Error("Wasm package integrity mismatch");
            bytes = null;
            controller.close();
          } else {
            if (loaded + value.byteLength > total) throw new Error("Unexpected Wasm package size");
            bytes.set(value, loaded);
            loaded += value.byteLength;
            report("download");
            controller.enqueue(value);
          }
        } catch (error) {
          controller.error(error);
        }
      },
      cancel(reason) { return reader.cancel(reason); },
    });
    const monitored = new Response(stream, { headers: { "Content-Type": "application/wasm" } });
    const module = typeof WebAssembly.compileStreaming === "function"
      ? await WebAssembly.compileStreaming(monitored)
      : await WebAssembly.compile(await monitored.arrayBuffer());
    report("starting", true);
    return module;
  } catch (error) {
    report("error", true);
    try { await reader?.cancel(error); } catch { /* already failed/cancelled */ }
    throw error;
  }
}
