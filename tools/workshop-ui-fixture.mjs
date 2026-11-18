// 工坊测试专用 IPC 夹具，与正式传输隔离。
export async function installWorkshopFixture(page, serial = false) {
  await page.evaluate(async (serial) => {
    const serviceUrl = performance
      .getEntriesByType("resource")
      .map((entry) => entry.name)
      .find((url) => new URL(url).pathname === "/src-ui/core/services/app.ts");
    if (!serviceUrl) throw Error("没有找到页面实际加载的服务模块");
    const { appService: api } = await import(serviceUrl);
    const original = api.command;
    const canvas = document.createElement("canvas");
    canvas.width = 500;
    canvas.height = 500;
    const ctx = canvas.getContext("2d");
    const gradient = ctx.createLinearGradient(0, 0, 500, 500);
    gradient.addColorStop(0, "#172d4f");
    gradient.addColorStop(1, "#815faf");
    ctx.fillStyle = gradient;
    ctx.fillRect(0, 0, 500, 500);
    ctx.strokeStyle = "#dfcaff";
    ctx.lineWidth = 3;
    ctx.beginPath();
    ctx.arc(250, 225, 140, 0, Math.PI * 2);
    ctx.stroke();
    ctx.fillStyle = "#e4d5ff";
    ctx.font = "90px sans-serif";
    ctx.textAlign = "center";
    ctx.fillText("✦", 250, 255);
    ctx.font = "28px sans-serif";
    ctx.fillText("工坊封面 · 验收夹具", 250, 435);
    const image = canvas.toDataURL();
    const file = (name, hash) => ({
      name,
      sourceName: "同名.png",
      canonical: "UT_HandCard_10001",
      sha256: hash.repeat(64),
      bytes: 70,
      resource: null,
    });
    const work = (id, title) => ({
      id,
      title,
      description: "社区卡面，保留透明",
      author: "星空作者",
      artist: "星空画师",
      authorUrls: ["https://example.com/author"],
      artistUrls: ["https://example.com/artist"],
      sourceUrl: "https://example.com/work",
      category: "卡面",
      tags: [{ name: "透明", highlight: false }],
      version: "1.0",
      gameVersion: "测试版本",
      changeNote: "",
      updatedAt: 0,
      revision: 1,
      cover: "cover.png",
      previews: [],
      history: [],
      archive: "content.zip",
      sha256: "0".repeat(64),
      bytes: 140,
      publishedId: null,
      coverDataUrl: null,
      files: [file("one.png", "a"), file("two.png", "b")],
    });
    const state = {
      listing: {
        available: true,
        status: "ready",
        message: "",
        works: [
          {
            work: work("new", "星空卡面"),
            installedVersion: null,
            locallyModified: false,
            updateAvailable: false,
          },
          {
            work: work("same", "同版本作品"),
            installedVersion: "1.0",
            locallyModified: false,
            updateAvailable: false,
          },
          {
            work: work("edit", "本地修改作品"),
            installedVersion: "1.0",
            locallyModified: true,
            updateAvailable: false,
          },
        ],
      },
      author: { drafts: [], works: [], receipts: [] },
      tasks: [],
      requests: [],
      covers: 0,
      available: false,
      links: [],
      exports: [],
      pickPaths: [],
      apx: {
        platformLabel: "国服 Steam",
        localPluginVersion: "0.3.0",
        remoteVersion: "0.4.0",
        installedFiles: 3,
        totalFiles: 3,
        directoryError: null,
        error: null,
        files: [],
      },
    };
    window.__workshopFixture = state;
    const delay = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
    api.openExternal = async (url) => {
      state.links.push(url);
    };
    api.chooseFiles = async () => state.pickPaths.shift() || null;
    api.saveFile = async (name) => `B:/APMM-next-work/fixture/${name}`;
    api.task = async (kind, title, args) => {
      state.requests.push({ kind, title, args });
      const id = crypto.randomUUID();
      state.tasks.push({
        id,
        title,
        kind,
        status: "running",
        phase: "upload",
        message: "夹具上传中",
        done: 0,
        total: 100,
        speed: null,
        revision: 1,
        canCancel: true,
        canRetry: false,
        result: null,
      });
      return id;
    };
    api.tasks = async () => structuredClone(state.tasks);
    api.command = async (command, args = {}) => {
      switch (command) {
        case "plugin_status":
          return structuredClone(state.apx);
        case "workshop_list":
          if (serial) for (let i = 0; i < 3; i++) await delay(600);
          return structuredClone(state.listing);
        case "workshop_image":
          state.covers++;
          if (args.name === "broken.png") throw Error("图片读取失败");
          await delay(600);
          return image;
        case "workshop_preflight": {
          const work = state.listing.works.find(
            (item) => item.work.id === args.id,
          ).work;
          return {
            work,
            workFingerprint: "server-work",
            localFingerprint: "server-local",
            canApply: true,
            entries: work.files.map((f) => ({
              ...f,
              currentHash: null,
              duplicate: false,
              disabled: false,
              gameHashes: [],
            })),
          };
        }
        case "author_image":
          return image;
        case "author_set_cover": {
          const d = state.author.drafts.find((d) => d.id === args.id);
          if (d.revision !== args.revision) throw Error("草稿已更新");
          if (args.path.includes("bad")) throw Error("图片解码失败");
          const next = {
            id: crypto.randomUUID(),
            sourceName: "cover.png",
            canonical: "",
            sha256: crypto.randomUUID().replaceAll("-", "").repeat(2),
            bytes: 70,
          };
          if (d.images.length) d.images[0] = next;
          else d.images.push(next);
          d.revision++;
          return structuredClone(d);
        }
        case "author_add_source": {
          const d = state.author.drafts.find((d) => d.id === args.id);
          d.images.push({
            id: crypto.randomUUID(),
            sourceName: "preview.png",
            canonical: "",
            sha256: crypto.randomUUID(),
            bytes: 70,
          });
          d.revision++;
          return structuredClone(d);
        }
        case "author_state":
          return structuredClone(state.author);
        case "author_export":
          state.exports.push(args);
          return null;
        case "author_service":
          return {
            available: state.available,
            message: state.available
              ? "投稿服务可用"
              : "投稿服务尚未配置；草稿与本地导出仍可使用",
          };
        case "author_new": {
          const d = {
            id: crypto.randomUUID(),
            revision: 0,
            workId: args.workId || null,
            metadata: {
              title: "",
              author: "",
              artist: "",
              authorUrls: [],
              artistUrls: [],
              sourceUrl: "",
              description: "",
              category: "",
              tags: [],
              version: "",
              gameVersion: "",
              changeNote: "",
            },
            files: [],
            images: [],
            metadataOnly: false,
            updatedAt: 0,
          };
          state.author.drafts.push(d);
          return structuredClone(d);
        }
        case "author_save": {
          await delay(250);
          const i = state.author.drafts.findIndex(
            (d) => d.id === args.draft.id,
          );
          if (state.author.drafts[i].revision !== args.draft.revision)
            throw Error("草稿已更新");
          state.author.drafts[i] = {
            ...structuredClone(args.draft),
            revision: args.draft.revision + 1,
          };
          return structuredClone(state.author.drafts[i]);
        }
        case "author_preflight": {
          const d = state.author.drafts.find((d) => d.id === args.id);
          return {
            id: crypto.randomUUID(),
            draft: structuredClone(d),
            bytes: 140,
            added: d.files.map((f) => f.sourceName),
            changed: [],
            removed: [],
            metadataChanged: true,
          };
        }
      }
      return original(command, args);
    };
  }, serial);
}
