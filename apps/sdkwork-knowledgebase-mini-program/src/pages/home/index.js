Page({
  data: { title: "SDKWork Knowledgebase" },
  openKnowledgebase() {
    wx.navigateTo({ url: "/pages/knowledgebase/index" });
  },
  openSearch() {
    wx.navigateTo({ url: "/pages/knowledgebase-search/index" });
  },
});
