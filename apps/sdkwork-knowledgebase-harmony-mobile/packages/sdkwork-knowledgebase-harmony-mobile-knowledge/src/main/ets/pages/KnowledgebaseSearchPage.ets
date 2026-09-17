/**
 * Knowledgebase search page.
 */
@Component
export struct KnowledgebaseSearchPage {
  @State message: string = 'Search';

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
