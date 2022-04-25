@Library('jenkins-library')

def pipeline = new org.docker.AppPipeline(steps: this,
    dockerImageName:              'iroha2-block-explorer-backend',
    dockerRegistryCred:           'bot-iroha2-rw',
    triggerCommonBuildExpression: false,
    secretScannerExclusion:       '.*Cargo.toml',
    nameCI:                       'iroha2-block-explorer-backend-CI')
pipeline.runPipeline()