@Library('jenkins-library')

def pipeline = new org.docker.AppPipeline(steps: this,
    dockerImageName:        'iroha2-block-explorer-backend',
    dockerRegistryCred:     'bot-iroha2-rw',
    secretScannerExclusion: '.*Cargo.toml')
pipeline.runPipeline()