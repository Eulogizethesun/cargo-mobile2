import { RustAbility } from '@ohos-rs/ability'
import Want from '@ohos.app.ability.Want'
import { AbilityConstant } from '@kit.AbilityKit';
import window from '@ohos.window';

export default class EntryAbility extends RustAbility {
  // The Rust library name (`lib<name>.so`), snake_cased from the app name
  // unless overridden in the app config.
  public moduleName: string = "{{#if app.lib-name}}{{app.lib-name}}{{else}}{{snake-case app.name}}{{/if}}"
  public defaultPage: boolean = true;
  public mode: 'xcomponent' | 'webview' = 'webview'

  async onCreate(want: Want, launchParam: AbilityConstant.LaunchParam): Promise<void> {
    super.onCreate(want, launchParam);
  }

  async onWindowStageCreate(windowStage: window.WindowStage): Promise<void> {
    const window = windowStage.getMainWindowSync();
    await window.setWindowLayoutFullScreen(false);
    super.onWindowStageCreate(windowStage);
  }
}
