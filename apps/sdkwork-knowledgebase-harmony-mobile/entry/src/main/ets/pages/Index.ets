import { getKnowledgebaseHarmonySdkClients } from '../bootstrap/SdkClients';

/**
 * Root page.
 *
 * The root page stays a thin shell mount point. Real screens are contributed by
 * capability packages (`packages/sdkwork-knowledgebase-harmony-mobile-*`).
 */
@Entry
@Component
struct Index {
  @State message: string = 'SDKWork Knowledgebase';

  aboutToAppear(): void {
    getKnowledgebaseHarmonySdkClients();
  }

  build() {
    Column() {
      Text(this.message)
        .fontSize(24)
        .fontWeight(FontWeight.Bold)
      Text('Harmony shell mount point')
        .fontSize(14)
        .fontColor('#666666')
        .margin({ top: 8 })
    }
    .width('100%')
    .height('100%')
    .justifyContent(FlexAlign.Center)
  }
}
