const { bootstrapKnowledgebaseMiniProgram } = require("../../runtime/knowledgebase-app");

Page({
  data: { title: "知识库详情", screenState: "loading" },
  onLoad() {
    try {
      bootstrapKnowledgebaseMiniProgram();
      this.setData({ screenState: "ready" });
    } catch {
      this.setData({ screenState: "unavailable" });
    }
  },
});
