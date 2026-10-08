pub const RPC_SDK_PROTOCOL: &str = "rpc";
pub const GENERATED_PROTO_ROOT: &str = "generated/proto";

// Protobuf codegen output (buf/prost) does not satisfy the generated-client lint
// baseline (qualified paths, clippy::all pedantics); scope the exemptions to the
// generated modules only so authored code keeps full coverage.
#[allow(warnings)]
pub mod sdkwork {
  pub mod common {
    pub mod v1 {
      include!(concat!(env!("CARGO_MANIFEST_DIR"), "/generated/proto/sdkwork/common/v1/sdkwork.common.v1.rs"));
    }
  }
  pub mod intelligence {
    pub mod internal {
      pub mod v1 {
        include!(concat!(env!("CARGO_MANIFEST_DIR"), "/generated/proto/sdkwork/intelligence/internal/v1/sdkwork.intelligence.internal.v1.rs"));
      }
    }
  }
}
