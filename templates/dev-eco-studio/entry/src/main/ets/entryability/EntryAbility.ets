import { NativeAbility } from '@ohos-rs/ability'
import Want from '@ohos.app.ability.Want'
import { AbilityConstant } from '@kit.AbilityKit';
import window from '@ohos.window';
import { PluginManager, Plugin } from '@tauri/app';

interface TauriNativeModule {
  default?: TauriNativeModule;
  tauri_init_plugins?: (manager: PluginManager) => string;
  tauriInitPlugins?: (manager: PluginManager) => string;
  tauri_handle_plugin_response?: (id: number, success: boolean, payload: string) => void;
  tauriHandlePluginResponse?: (id: number, success: boolean, payload: string) => void;
}

interface PluginConfig {
  name: string;
  identifier: string;
  className: string;
  config: string;
}

// Static plugin registry - add your plugins here
// Example: import DialogPlugin from '@tauri/plugin-dialog';
// const STATIC_PLUGINS: Map<string, Plugin> = new Map();
// STATIC_PLUGINS.set('dialog', new DialogPlugin());
const STATIC_PLUGINS: Map<string, Plugin> = new Map();

export default class EntryAbility extends NativeAbility {
  public moduleName: string = "{{app.name}}"
  public defaultPage: boolean = true;
  public mode: 'xcomponent' | 'webview' = 'webview'

  async onCreate(want: Want, launchParam: AbilityConstant.LaunchParam): Promise<void> {
    console.info('[Tauri] ===== EntryAbility onCreate =====');
    await super.onCreate(want, launchParam);
    console.info('[Tauri] ===== super.onCreate done =====');
    await this.initTauriPlugins();
  }

  private async initTauriPlugins(): Promise<void> {
    console.info('[Tauri] ===== initTauriPlugins =====');

    const pluginManager = new PluginManager();
    console.info('[Tauri] PluginManager created');

    pluginManager.handlePluginResponse = (id: number, success: number, payload: string): void => {
      const libName = 'lib' + this.moduleName + '.so';
      import(libName).then((nativeModule: TauriNativeModule): void => {
        const actualModule = nativeModule.default ? nativeModule.default : nativeModule;
        const handler = actualModule?.tauri_handle_plugin_response ?? actualModule?.tauriHandlePluginResponse;
        if (handler != null) {
          handler(id, success === 1, payload);
        }
      }).catch((err: Error): void => {
        console.error('[Tauri] Failed to call handlePluginResponse: ' + err.message);
      });
    };
    
    // Also set the global response handler via the setResponseHandler method
    pluginManager.setResponseHandler(pluginManager.handlePluginResponse);

    const libName = 'lib' + this.moduleName + '.so';
    console.info('[Tauri] Loading: ' + libName);

    try {
      const nativeModule = await import(libName) as TauriNativeModule;
      console.info('[Tauri] Native module loaded');
      console.info('[Tauri] Module keys: ' + Object.keys(nativeModule).join(', '));
      
      const actualModule = nativeModule.default ? nativeModule.default : nativeModule;
      console.info('[Tauri] Actual module keys: ' + Object.keys(actualModule).join(', '));

      if (actualModule?.tauri_init_plugins != null || actualModule?.tauriInitPlugins != null) {
        const initPlugins = actualModule.tauri_init_plugins ?? actualModule.tauriInitPlugins;
        if (initPlugins != null) {
          console.info('[Tauri] Calling initPlugins');
          const pluginsJson = initPlugins(pluginManager);
          const plugins: PluginConfig[] = JSON.parse(pluginsJson) as PluginConfig[];

          console.info('[Tauri] Loading ' + plugins.length + ' plugins');

          for (const plugin of plugins) {
            const staticPlugin = STATIC_PLUGINS.get(plugin.name);
            if (staticPlugin != null) {
              pluginManager.load(plugin.name, staticPlugin, plugin.config);
              console.info('[Tauri] Plugin loaded: ' + plugin.name);
            } else {
              console.error('[Tauri] Plugin not in static registry: ' + plugin.name);
            }
          }
          console.info('[Tauri] All plugins loaded');
        }
      } else {
        console.warn('[Tauri] initPlugins not found in module');
      }
    } catch (e) {
      const err = e as Error;
      console.error('[Tauri] Error: ' + err.message);
      console.error('[Tauri] Stack: ' + err.stack);
    }
  }

  async onWindowStageCreate(windowStage: window.WindowStage): Promise<void> {
    console.info('[Tauri] ===== onWindowStageCreate =====');
    const win = windowStage.getMainWindowSync();
    await win.setWindowLayoutFullScreen(false);
    await super.onWindowStageCreate(windowStage);
    console.info('[Tauri] ===== super.onWindowStageCreate done =====');
  }
}
