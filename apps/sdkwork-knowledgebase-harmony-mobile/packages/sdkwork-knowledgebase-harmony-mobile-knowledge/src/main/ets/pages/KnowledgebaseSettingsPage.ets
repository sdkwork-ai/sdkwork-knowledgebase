/**
 * Knowledgebase settings page.
 */
@Component
export struct KnowledgebaseSettingsPage {
  @State message: string = 'Settings';

  build() {
    Column() {
      Text(this.message)
        .fontSize(20)
        .fontWeight(FontWeight.Medium)
    }
    .width('100%')
    .height('100%')
    .justifyContent(FlexAlign.Center)
  }
}
