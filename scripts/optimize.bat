docker run --rm -v "%cd%":/code --mount type=volume,source=arena_cache,target=/code/target --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry cosmwasm/workspace-optimizer:0.14.0