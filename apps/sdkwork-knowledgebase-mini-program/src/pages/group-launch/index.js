const { bootstrapKnowledgebaseMiniProgram } = require("../../runtime/knowledgebase-app");

Page({
  data: { title: "群知识库", screenState: "loading" },
  onLoad() {
    try {
      bootstrapKnowledgebaseMiniProgram();
      this.setData({ screenState: "ready" });
    } catch {
      this.setData({ screenState: "unavailable" });
    }
  },
});
