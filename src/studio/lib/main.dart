import 'package:flutter/material.dart';

void main() {
  runApp(const LabApp());
}

/// 应用入口与骨架。
///
/// 参考 qtfounder studio 的最小壳；业务代码在 `lib/` 下按功能分层添加。
class LabApp extends StatelessWidget {
  const LabApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: '量潮创始人实验室',
      debugShowCheckedModeBanner: false,
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(
          seedColor: const Color(0xFF4F46E5),
          surface: Colors.white,
          brightness: Brightness.light,
        ),
        scaffoldBackgroundColor: const Color(0xFFF1F5F9),
        useMaterial3: true,
        fontFamily: 'Inter',
      ),
      home: const Scaffold(body: Center(child: Text('量潮创始人实验室'))),
    );
  }
}
