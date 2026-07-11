#
# To learn more about a Podspec see http://guides.cocoapods.org/syntax/podspec.html.
# Run `pod lib lint wecode_protocol_ffi.podspec` to validate before publishing.
#
Pod::Spec.new do |s|
  s.name             = 'wecode_protocol_ffi'
  s.version          = '1.8.0'
  s.summary          = 'Rust-backed WeCode remote protocol bindings.'
  s.description      = <<-DESC
WeCode remote protocol FFI bridge backed by the shared Rust protocol crate.
                       DESC
  s.homepage         = 'https://wecode.dev'
  s.license          = { :file => '../LICENSE' }
  s.author           = { 'WeCode' => 'support@wecode.dev' }

  s.source           = { :path => '.' }
  s.source_files = 'Classes/**/*'
  s.preserve_paths = '../scripts/**/*', 'Frameworks/**/*'
  s.dependency 'Flutter'
  s.platform = :ios, '13.0'
  s.script_phase = {
    :name => 'Build WeCode Protocol Rust FFI',
    :script => 'bash "$PODS_TARGET_SRCROOT/../scripts/build-apple.sh"',
    :execution_position => :before_compile
  }

  # Flutter.framework does not contain a i386 slice.
  s.pod_target_xcconfig = {
    'DEFINES_MODULE' => 'YES',
    'EXCLUDED_ARCHS[sdk=iphonesimulator*]' => 'i386',
    'LIBRARY_SEARCH_PATHS' => '$(inherited) "${PODS_TARGET_SRCROOT}/Frameworks"'
  }
  s.user_target_xcconfig = {
    'OTHER_LDFLAGS' => '$(inherited) -force_load "${PODS_ROOT}/../.symlinks/plugins/wecode_protocol_ffi/ios/Frameworks/libwecode_protocol_ffi.a"'
  }
  s.swift_version = '5.0'
end
