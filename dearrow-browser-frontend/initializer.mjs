export default function() {
  return {
    onStart: () => {
      const detail = document.getElementById("load-detail");
      detail.innerText = "we're starting the WASM download";
    },
    onProgress: ({current, total}) => {
      const progress = document.getElementById("load-progress");
      const detail = document.getElementById("load-detail");
        const current_kB = Math.floor(current / 100) / 10;

      if (total != 0) {
        const total_kB = Math.floor(total / 100) / 10;
        progress.max = total;
        progress.value = current;
        detail.innerText = `${current_kB} kB / ${total_kB} kB`;
      } else {
        detail.innerText = `${current_kB} kB / nobody knows`;
      }
    },
    onComplete: () => {},
    onSuccess: () => {
      document.getElementById("loading")?.remove();
    },
    onFailure: e => {
      document.getElementById("load-detail").remove();
      document.getElementById("load-progress").remove();
      document.getElementById("load-header").innerText = "DeArrow Browser failed to initialize";
      document.getElementById("load-text").innerText = `Make sure your browser supports WASM and try hard-refreshing (usually by clicking reload while holding shift)\n\nDetails about the error should appear below.`;
      const err_detail = document.createElement("pre");
      err_detail.innerText = `${e.toString()}\n\nStack trace:\n${e.stack}`;
      document.getElementById("loading").appendChild(err_detail);
    }
  }
}
