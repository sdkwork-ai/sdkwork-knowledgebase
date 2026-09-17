/**
 * Knowledgebase document detail page.
 */
@Component
export struct KnowledgebaseDetailPage {
  @State message: string = 'Document';

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
