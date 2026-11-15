// 仅开发服务器读取指定本地资源；传给浏览器的图片只使用受限的同源路径。
import fs from "node:fs";
import path from "node:path";
import type { Plugin } from "vite";
import catalog from "../config/resources.json";
export function localPreview(): Plugin {
  const fallback =
    "B:/APMM-next-work/validation-26b2be76-499e-48ee-83df-b9edaeff56b0/assets";
  const data = path.resolve("dist/APMM-next/data");
  const mods = new Map<string, string>();
  const images = new Map<string, string>();
  function snapshot() {
    const library = JSON.parse(
      fs.readFileSync(path.join(data, "library.json"), "utf8"),
    );
    const rows: any[] = structuredClone(catalog);
    const byKey = new Map(rows.map((r) => [r.key, r]));
    for (const r of rows) {
      const roots = [
        process.env.APMM_PREVIEW_ROOT,
        path.join(data, "assets/outimg"),
        path.join(fallback, "outimg"),
      ].filter(Boolean) as string[];
      const file = roots
        .map((root) =>
          path.join(root, path.basename(r.previewFile || `${r.canonical}.png`)),
        )
        .find(fs.existsSync);
      r.originalUrl = file ? `/__preview/${r.canonical}.png` : null;
      r.originalStatus = file ? "local" : "not-extracted";
      if (file) images.set(`${r.canonical}.png`, file);
    }
    const vault = fs.realpathSync(path.join(data, "vault"));
    for (const entities of [library.heroes, library.monsters])
      for (const [id, entity] of Object.entries(entities || {}) as any[]) {
        for (const [section, sec] of Object.entries(
          entity.sections || {},
        ) as any[])
          for (const [canonical, resource] of Object.entries(
            sec.resources || {},
          ) as any[]) {
            const key = `${section}/${canonical}`;
            let row = byKey.get(key);
            if (!row) {
              row = {
                key,
                entityId: id,
                entityName: `未收录名称 · ${id}`,
                nameKnown: false,
                aliases: [],
                category: section.startsWith("monster")
                  ? "monsters"
                  : "temporary",
                section,
                sectionLabel: section,
                resourceLabel: section,
                canonical,
                fullName: canonical,
                applicationSlot: canonical,
                groupKey: key,
                variantKey: "base",
                variantLabel: "原皮",
                variantOrder: 0,
                originalUrl: null,
                originalStatus: "not-extracted",
                files: [],
                activeHash: null,
                disabled: false,
              };
              rows.push(row);
            }
            row.activeHash = resource.activeHash || null;
            row.disabled = !!resource.disabled;
            row.files = (resource.files || []).flatMap((f: any) => {
              const target = path.isAbsolute(f.relPath || "")
                ? f.relPath
                : path.join(vault, f.relPath || `${f.hash}${f.ext || ".png"}`);
              if (!fs.existsSync(target))
                return [{ hash: f.hash, sourceName: f.sourceName, path: null }];
              const real = fs.realpathSync(target);
              if (!real.startsWith(vault + path.sep)) return [];
              const name = `${f.hash}.png`;
              mods.set(name, real);
              return [
                {
                  hash: f.hash,
                  sourceName: f.sourceName,
                  path: `/__mod/${name}`,
                },
              ];
            });
          }
      }
    return {
      resources: rows,
      gameDir: library.settings?.modimgPath
        ? path.dirname(library.settings.modimgPath)
        : null,
    };
  }
  return {
    name: "local-preview",
    configureServer(server) {
      server.middlewares.use((req, res, next) => {
        const url = (req.url || "").split("?")[0];
        if (url === "/__workspace") {
          try {
            res.setHeader("Content-Type", "application/json");
            res.end(JSON.stringify(snapshot()));
          } catch (e) {
            res.statusCode = 500;
            res.end(String(e));
          }
          return;
        }
        const match = /^\/__(preview|mod)\/([A-Za-z0-9_]+\.png)$/.exec(url);
        if (!match) return next();
        const file = (match[1] === "mod" ? mods : images).get(match[2]);
        if (!file) {
          res.statusCode = 404;
          res.end();
          return;
        }
        res.setHeader("Content-Type", "image/png");
        fs.createReadStream(file).pipe(res);
      });
    },
  };
}
