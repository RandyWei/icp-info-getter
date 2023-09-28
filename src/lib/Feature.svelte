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

  enum Status {
    Default,
    Loading,
    Result,
    Error,
  }

  let currentStatus: Status = Status.Default;

  let appName: string = "";
  let icon: string = "";
  let bundleId: string = "";
  let sha1: string = "";
  let modulus: string = "";

  async function releaseListener() {
    fileDropListener();
  }

  async function setupEventListener() {
    fileDropListener = await tauriWindow.appWindow.onFileDropEvent(
      async (event) => {
        if (event.payload.type == "drop" && event.payload.paths.length > 0) {
          const filePath = event.payload.paths[0];
          if (!filePath.toLocaleLowerCase().endsWith("ipa")) {
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

          currentStatus = Status.Loading;

          const cacheDir = await path.appCacheDir();

          invoke("parse", {
            ipaPath: filePath,
            cachePath: cacheDir,
          })
            .then((s) => {
              appName = s.name;
              icon = s.icon;
              bundleId = s.bundle_id;
              sha1 = s.sha1;
              modulus = s.modulus;
              console.log(s);
              console.log("调用完成");
              currentStatus = Status.Result;
            })
            .catch((e) => {
              console.error(e);
              currentStatus = Status.Error;
            });
        }
      }
    );
  }

  onMount(async () => {
    setupEventListener();
  });

  onDestroy(() => {
    releaseListener();
  });
</script>

<div id="container">
  {#if currentStatus == Status.Default}
    <div id="tip">请将ipa包拖进来</div>
  {:else if currentStatus == Status.Loading}
    <div id="tip">正在解析中</div>
  {:else if currentStatus == Status.Error}
    <div id="tip">解析失败，请重新尝试</div>
  {:else}
    <div id="result">
      <div class="line"><img src="data:image/png;base64,{icon}" alt="" /></div>
      <div class="line">APP名称：{appName}</div>
      <div class="line">Bundle Id：{bundleId}</div>
      <div class="line">证书MD5指纹(签名MD5值、sha-1)：{sha1}</div>
      <div class="line">Modulus(公钥)：{modulus}</div>
    </div>
  {/if}
  <div><button>保存</button></div>
</div>

<style>
  #container {
    width: 80%;
    height: 100%;
    min-height: 100%;
    text-align: left;
    word-break: break-all;
  }
  #tip {
    font-size: xx-large;
    height: 100vh;
    text-align: center;
    align-items: center;
    justify-content: center;
    display: flex;
  }
  .line {
    padding: 0.5rem 0;
  }
</style>
