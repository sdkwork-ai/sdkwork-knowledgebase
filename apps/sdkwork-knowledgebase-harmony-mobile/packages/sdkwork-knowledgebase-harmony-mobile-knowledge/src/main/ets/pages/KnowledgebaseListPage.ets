/**
 * Knowledgebase catalog list page.
 */
@Component
export struct KnowledgebaseListPage {
  @State message: string = 'Knowledgebase';

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
