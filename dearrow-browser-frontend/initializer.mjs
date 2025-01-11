/* This file is part of the DeArrow Browser project - https://github.com/mini-bomba/DeArrowBrowser
*
*  Copyright (C) 2024-2025 mini_bomba
*  
*  This program is free software: you can redistribute it and/or modify
*  it under the terms of the GNU Affero General Public License as published by
*  the Free Software Foundation, either version 3 of the License, or
*  (at your option) any later version.
*
*  This program is distributed in the hope that it will be useful,
*  but WITHOUT ANY WARRANTY; without even the implied warranty of
*  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
*  GNU Affero General Public License for more details.
*
*  You should have received a copy of the GNU Affero General Public License
*  along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/
export default function() {
  const speedHistory = [];
  let lastProgress = 0;
  return {
    onStart: () => {
      const detail = document.getElementById("load-detail");
      detail.innerText = "we're starting the WASM download";
    },
    onProgress: ({current, total}) => {
      // update the speed history window
      const now = performance.now();
      const diff = current-lastProgress;
      lastProgress = current;
      speedHistory.push({time: now, diff});
      while (speedHistory.length > 21) speedHistory.shift();

      const progress = document.getElementById("load-progress");
      const detail = document.getElementById("load-detail");
        const current_kB = Math.floor(current / 100) / 10;

      if (total >= 0) {
        const total_kB = Math.floor(total / 100) / 10;
        progress.max = total;
        progress.value = current;
        detail.innerText = `${current_kB} kB / ${total_kB} kB`;
      } else {
        detail.innerText = `${current_kB} kB / nobody knows`;
      }
      if (speedHistory.length > 1) {
        const diff = speedHistory.reduce((acc, cur) => acc+cur.diff, 0) - speedHistory[0].diff;
        const time = speedHistory[speedHistory.length-1].time - speedHistory[0].time;
        const speed = diff / time * 1000;
        const speed_kB = Math.floor(diff / time * 10) / 10;
        if (total >= 0) {
          const left = total-current;
          const eta = Math.ceil(left/speed);
          detail.innerText += `\n~${speed_kB} kB/s, eta ${eta}s`;
        } else {
          detail.innerText += `\n~${speed_kB} kB/s`;
        }
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
