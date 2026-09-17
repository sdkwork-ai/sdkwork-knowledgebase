/**
 * Group knowledgebase launch page. The only input is the opaque launch ticket; the page never reads space identifiers, destinations, session tokens, or caller context from the launch payload.
 */
@Component
export struct GroupKnowledgebaseLaunchPage {
  @State message: string = 'Group knowledgebase';

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
