const { bootstrapKnowledgebaseMiniProgram } = require("./runtime/knowledgebase-app");
const runtimeEnv = require("./runtime/runtime-env");

App({
  globalData: {
    sdkworkProfileId: runtimeEnv.SDKWORK_PROFILE_ID,
    knowledgebaseAppApiBaseUrl: runtimeEnv.SDKWORK_KNOWLEDGEBASE_MP_APP_API_BASE_URL,
  },
  onLaunch() {
    try {
      bootstrapKnowledgebaseMiniProgram({
        appApiBaseUrl: this.globalData.knowledgebaseAppApiBaseUrl,
      });
    } catch {
      // The runtime bundle is produced by pnpm build:mini-program.
    }
    wx.reLaunch({ url: "/pages/home/index" });
  },
});
