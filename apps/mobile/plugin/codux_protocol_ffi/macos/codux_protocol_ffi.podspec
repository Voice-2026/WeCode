#
# To learn more about a Podspec see http://guides.cocoapods.org/syntax/podspec.html.
# Run `pod lib lint codux_protocol_ffi.podspec` to validate before publishing.
#
Pod::Spec.new do |s|
  s.name             = 'codux_protocol_ffi'
  s.version          = '1.8.0'
  s.summary          = 'Rust-backed Codux remote protocol bindings.'
  s.description      = <<-DESC
Codux remote protocol FFI bridge backed by the shared Rust protocol crate.
                       DESC
  s.homepage         = 'https://codux.dev'
  s.license          = { :file => '../LICENSE' }
  s.author           = { 'Codux' => 'support@codux.dev' }

  s.source           = { :path => '.' }
  s.source_files = 'Classes/**/*'
  s.preserve_paths = '../scripts/**/*', 'Frameworks/**/*'
  s.script_phase = {
    :name => 'Build Codux Protocol Rust FFI',
    :script => 'bash "$PODS_TARGET_SRCROOT/../scripts/build-apple.sh"',
    :execution_position => :before_compile
  }

  s.dependency 'FlutterMacOS'

  s.platform = :osx, '10.11'
  s.pod_target_xcconfig = {
    'DEFINES_MODULE' => 'YES',
    'LIBRARY_SEARCH_PATHS' => '$(inherited) "${PODS_TARGET_SRCROOT}/Frameworks"'
  }
  s.user_target_xcconfig = {
    'OTHER_LDFLAGS' => '$(inherited) -force_load "${PODS_ROOT}/../.symlinks/plugins/codux_protocol_ffi/macos/Frameworks/libcodux_protocol_ffi.a"'
  }
  s.swift_version = '5.0'
end
