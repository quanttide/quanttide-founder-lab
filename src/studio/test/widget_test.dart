import 'package:flutter_test/flutter_test.dart';

import 'package:qtfounder_studio_lab/main.dart';

void main() {
  testWidgets('骨架可渲染', (WidgetTester tester) async {
    await tester.pumpWidget(const LabApp());

    expect(find.text('量潮创始人实验室'), findsOneWidget);
  });
}
