import toml
with open(".deepsource.toml") as f:
    print(toml.load(f))
