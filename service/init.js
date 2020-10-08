import "./install.js";
import "./verify.js";

navigator.serviceWorker.ready.then(() => {
  // A ServiceWorker can't create web workers nor does it have any other way to
  // create unsoiled execution contexts, so it has to obtain Workers from a
  // Window object.
  navigator.serviceWorker.addEventListener("message", (event) => {
    switch (event.data.op) {
      case "newWorker": {
        const {
          data: { url, options, message },
          ports,
        } = event;
        const worker = new Worker(url, options);
        worker.postMessage(message, ports);
        break;
      }

      // Currently unused.
      case "newIframe": {
        const {
          data: { url, message },
          ports,
        } = event;
        const iframe = document.createElement("iframe");
        iframe.setAttribute("src", url);
        iframe.style.visibility = "hidden";
        document.body.appendChild(iframe);
        iframe.addEventListener(
          "load",
          () => {
            iframe.contentWindow.postMessage(message, ports);
          },
          { once: true }
        );
        break;
      }

      default:
        throw new TypeError(`Unknown op: '${op}'`);
    }
  });
  navigator.serviceWorker.startMessages();

  // As the loader setup is now complete, the "real" top level imported
  // modules can be loaded. When initially designated with
  // `<script type="denodule" src="..."></script>`, here we change the
  // script type to "module" so these scripts will be actually loaded.
  const rootModuleTags = document.querySelectorAll('script[type="denodule"]');
  for (const tag of rootModuleTags) {
    const tag2 = tag.cloneNode(true);
    tag2.setAttribute("type", "module");
    tag.parentNode.replaceChild(tag2, tag);
  }
});
