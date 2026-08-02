import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

// `web.js` imports the bridge through `../makepad_wasm_bridge/`, a path that
// only exists in the packaged web layout. Load the module with that one import
// replaced by a local stub so the handlers stay testable from the repository.
async function load_web_browser() {
    const source = await readFile(new URL("./web.js", import.meta.url), "utf8");
    const patched = source.replace(
        /^import \{ WasmBridge \}.*$/m,
        "class WasmBridge { constructor() {} }",
    );
    assert.ok(
        patched.includes("class WasmBridge { constructor() {} }"),
        "the wasm bridge import must be stubbed",
    );
    const module = await import(
        `data:text/javascript,${encodeURIComponent(patched)}`
    );
    return module.WasmWebBrowser;
}

function install_dom_stubs() {
    const calls = {
        blobs: [],
        created: [],
        appended: [],
        removed: [],
        clicks: 0,
        object_urls: [],
        revoked: [],
    };

    class FakeBlob {
        constructor(parts, options) {
            this.parts = parts;
            this.type = options && options.type;
            calls.blobs.push(this);
        }
    }

    const anchor = {
        href: undefined,
        download: undefined,
        style: {},
        click() {
            calls.clicks += 1;
        },
        remove() {
            calls.removed.push(anchor);
        },
    };

    const previous = {
        Blob: globalThis.Blob,
        URL: globalThis.URL,
        document: globalThis.document,
    };

    globalThis.Blob = FakeBlob;
    globalThis.URL = {
        createObjectURL(blob) {
            const url = `blob:fake/${calls.object_urls.length}`;
            calls.object_urls.push({ url, blob });
            return url;
        },
        revokeObjectURL(url) {
            calls.revoked.push(url);
        },
    };
    globalThis.document = {
        createElement(tag) {
            calls.created.push(tag);
            return anchor;
        },
        body: {
            appendChild(node) {
                calls.appended.push(node);
            },
            removeChild(node) {
                calls.removed.push(node);
            },
        },
    };

    return {
        calls,
        anchor,
        restore() {
            globalThis.Blob = previous.Blob;
            globalThis.URL = previous.URL;
            globalThis.document = previous.document;
        },
    };
}

test("FromWasmDownloadFile delivers one clicked blob anchor and revokes its url", async () => {
    const WasmWebBrowser = await load_web_browser();
    const dom = install_dom_stubs();
    try {
        const bridge = new WasmWebBrowser(undefined, undefined, undefined);
        const data = new Uint8Array([1, 2, 3]);

        bridge.FromWasmDownloadFile({
            name: "orders.waml",
            mime_type: "application/vnd.waml.bundle",
            data,
        });

        assert.equal(dom.calls.blobs.length, 1, "one blob is created");
        assert.equal(dom.calls.blobs[0].type, "application/vnd.waml.bundle");
        assert.deepEqual(dom.calls.blobs[0].parts, [data]);

        assert.deepEqual(dom.calls.created, ["a"]);
        assert.equal(dom.anchor.download, "orders.waml");
        assert.equal(dom.anchor.href, dom.calls.object_urls[0].url);
        assert.equal(dom.calls.clicks, 1, "the anchor is clicked exactly once");
        assert.ok(dom.calls.removed.includes(dom.anchor), "the anchor is removed");

        await Promise.resolve();
        await Promise.resolve();
        assert.deepEqual(dom.calls.revoked, [dom.calls.object_urls[0].url]);
    } finally {
        dom.restore();
    }
});
