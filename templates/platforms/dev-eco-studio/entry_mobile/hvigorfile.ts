import { hapTasks } from "@ohos/hvigor-ohos-plugin";
import { hvigor, HvigorPlugin, HvigorNode } from "@ohos/hvigor";
import { execFileSync } from "child_process";
import { resolve } from "path";

export default {
  system: hapTasks /* Built-in plugin of Hvigor. It cannot be modified. */,
  plugins: [
    cargoMobilePlugin(),
  ] /* Custom plugin to extend the functionality of Hvigor. */,
};

function cargoMobilePlugin(): HvigorPlugin {
  return {
    pluginId: "cargo-mobile",
    apply(node: HvigorNode) {
      const buildRustCode = () => {
        // Bake this entry module's form so the `.so` compiled by
        // `cargo open-harmony build` is distributed into this module's
        // `libs/` dir (and gets the right `cfg(mobile)`/`cfg(desktop)`
        // aliases when the app uses them).
        process.env.OHOS_DEVICE_TYPE = "mobile";
        const properties = hvigor.getParameter().getProperties();
        const target = properties.target || "aarch64";
        const args = ["open-harmony", "build", target.toString()];
        // Keep the Rust profile in sync with the hvigor build mode, so a
        // release HAP doesn't silently package a debug `.so`.
        if (properties.buildMode === "release") {
          args.push("--release");
        }
        execFileSync(`cargo`, args, {
          cwd: resolve(__dirname, "{{root-dir-rel}}"),
          stdio: "inherit",
        });
      };

      node.getTaskByName("default@ConfigureCmake")!.afterRun(buildRustCode);
    },
  };
}
