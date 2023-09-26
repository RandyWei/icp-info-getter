<script lang="ts">
  import {
    http,
    os,
    window as tauriWindow,
    event,
    fs,
    invoke,
    path,
    dialog,
    shell,
  } from "@tauri-apps/api";

  import { onMount, onDestroy } from "svelte";
  import { toast } from "@zerodevx/svelte-toast";

  let fileDropListener: event.UnlistenFn;

  async function releaseListener() {
    fileDropListener();
  }

  async function setupEventListener() {
    fileDropListener = await tauriWindow.appWindow.onFileDropEvent((event) => {
      if (event.payload.type == "drop" && event.payload.paths.length > 0) {
        const path = event.payload.paths[0];
        if (!path.toLocaleLowerCase().endsWith("ipa")) {
          toast.push("请拖入ipa包", {
            theme: {
              "--toastBarHeight": 0,
              "--toastColor": "mintcream",
              "--toastBackground": "rgba(255,0,0,0.9)",
              "--toastBarBackground": "red",
            },
          });
          return;
        }
      }
    });
  }

  onMount(async () => {
    setupEventListener();
  });

  onDestroy(() => {
    releaseListener();
  });
</script>

<div />
